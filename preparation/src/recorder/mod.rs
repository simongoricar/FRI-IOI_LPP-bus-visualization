use std::{
    collections::{HashMap, HashSet},
    error::Error,
    fs::OpenOptions,
    future::Future,
    io::{BufWriter, Write},
    path::Path,
    time::{Duration, Instant},
};

use backoff::{backoff::Backoff, exponential::ExponentialBackoff, ExponentialBackoffBuilder};
use chrono::Utc;
use miette::{miette, Context, Diagnostic, IntoDiagnostic, Result};
use reqwest::Client;
use serde::Serialize;
use thiserror::Error;
use tokio::task::yield_now;
use tracing::{debug, error, info, info_span, warn, Instrument};

pub mod formats;

use crate::{
    api::{
        routes::fetch_all_routes,
        routes_on_station::fetch_routes_on_station,
        station_details::fetch_station_details,
        stations_on_route::fetch_stations_on_route,
        timetable::{fetch_timetable, TimetableFetchMode, TripTimetable},
        BusRoute,
        StationCode,
    },
    cancellation_token::CancellationToken,
    cli::RunMode,
    configuration::LppConfiguration,
    recorder::formats::{
        AllRoutesSnapshot,
        AllStationsSnapshot,
        StationDetailsWithBusesAndTimetables,
        TripStationWithTimetable,
        TripWithStationsAndTimetables,
    },
    storage::{RouteStorage, StationStorage},
};


fn save_json_to_file<S>(data: &S, file_path: &Path) -> Result<()>
where
    S: Serialize,
{
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(file_path)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to open file for writing."))?;

    let mut buf_writer = BufWriter::new(file);


    serde_json::to_writer(&mut buf_writer, data)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to write JSON data to file."))?;


    let mut file = buf_writer
        .into_inner()
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to flush output file's BufWriter."))?;

    file.flush()
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to flush output file."))?;

    Ok(())
}


/*
 * Station and route details capture
 */

async fn make_station_and_route_snapshot(
    configuration: &LppConfiguration,
    client: &Client,
    station_storage: &StationStorage,
    route_storage: &RouteStorage,
) -> Result<()> {
    // Fetch all stations.
    let stations = retryable_async_with_exponential_backoff(
        || fetch_station_details(&configuration.api, client),
        |result| match result {
            Ok(details) => RetryableResult::Ok(details),
            Err(error) => RetryableResult::TransientErr {
                error,
                override_retry_after: None,
            },
        },
        None,
    )
    .instrument(info_span!("station-details"))
    .await
    .into_diagnostic()
    .wrap_err_with(|| miette!("Failed to fetch station details."))?;


    // For each station, get all buses (trips) that stop there.
    let mut bus_trip_to_timetable: HashMap<BusRoute, HashMap<StationCode, TripTimetable>> =
        HashMap::new();

    let mut stations_with_bus_trips = Vec::with_capacity(stations.len());

    let total_number_of_stations = stations.len();

    for (station_index, station) in stations.into_iter().enumerate() {
        debug!(
            current_station = station_index + 1,
            total_stations = total_number_of_stations,
            station_name = station.name,
            station_code = %station.station_code,
            "Requesting routes on station."
        );

        let trips_on_station = retryable_async_with_exponential_backoff(
            || fetch_routes_on_station(&configuration.api, client, &station.station_code),
            |result| match result {
                Ok(details) => RetryableResult::Ok(details),
                Err(error) => RetryableResult::TransientErr {
                    error,
                    override_retry_after: None,
                },
            },
            None,
        )
        .instrument(info_span!("trips-on-station"))
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to fetch trips on station."))?;



        let mut all_route_groups = HashSet::new();
        for trip in &trips_on_station {
            all_route_groups.insert(trip.route.to_base_route());
        }


        if all_route_groups.is_empty() {
            debug!(
                current_station = station_index + 1,
                total_stations = total_number_of_stations,
                station_name = station.name,
                station_code = %station.station_code,
                "Station has no route groups, will not request a timetable."
            );
            continue;
        }


        debug!(
            current_station = station_index + 1,
            total_stations = total_number_of_stations,
            station_name = station.name,
            station_code = %station.station_code,
            "Requesting full timetable for station."
        );

        let timetables = retryable_async_with_exponential_backoff(
            || {
                fetch_timetable(
                    &configuration.api,
                    client,
                    &station.station_code,
                    all_route_groups.clone(),
                    TimetableFetchMode::FullDay,
                )
            },
            |result| match result {
                Ok(details) => RetryableResult::Ok(details),
                Err(error) => RetryableResult::TransientErr {
                    error,
                    override_retry_after: None,
                },
            },
            None,
        )
        .instrument(info_span!("timetable-on-station"))
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to fetch timetables on station."))?;


        // Add the timetables into a hash map for later access (when we'll assign timetables to bus trips).
        for group_timetable in &timetables {
            for trip_timetable in &group_timetable.trip_timetables {
                if let Some(trips_map) = bus_trip_to_timetable.get_mut(&trip_timetable.route) {
                    trips_map.insert(
                        station.station_code.clone(),
                        trip_timetable.clone(),
                    );
                } else {
                    let mut map = HashMap::new();
                    map.insert(
                        station.station_code.clone(),
                        trip_timetable.clone(),
                    );

                    bus_trip_to_timetable.insert(trip_timetable.route.clone(), map);
                }
            }
        }


        let station_with_trips = StationDetailsWithBusesAndTimetables::from_station_and_trips(
            station,
            trips_on_station,
            timetables,
        );

        stations_with_bus_trips.push(station_with_trips);
    }


    // Now we'll fetch all bus routes and assign them a trip timetable.
    debug!("Requesting all routes.");

    let all_routes = retryable_async_with_exponential_backoff(
        || fetch_all_routes(&configuration.api, client),
        |result| match result {
            Ok(details) => RetryableResult::Ok(details),
            Err(error) => RetryableResult::TransientErr {
                error,
                override_retry_after: None,
            },
        },
        None,
    )
    .instrument(info_span!("all-routes"))
    .await
    .into_diagnostic()
    .wrap_err_with(|| miette!("Failed to fetch all routes."))?;


    let mut routes_with_context = Vec::with_capacity(all_routes.len());

    let number_of_all_routes = all_routes.len();

    for (route_index, route) in all_routes.into_iter().enumerate() {
        let captured_at = Utc::now();


        let raw_route_timetables = match bus_trip_to_timetable.get(&route.route) {
            Some(timetable_map) => timetable_map,
            None => {
                // It's possible that we have some bad data that has
                // no associated timetable data. In this case, we ignore the route.
                warn!(
                    current_route = route_index + 1,
                    total_routes = number_of_all_routes,
                    route = %route.route,
                    "Did not collect any timetables for this route - will skip."
                );
                continue;
            }
        };


        debug!(
            current_route = route_index + 1,
            total_routes = number_of_all_routes,
            "Requesting stations on route."
        );

        let stations_on_route = retryable_async_with_exponential_backoff(
            || fetch_stations_on_route(&configuration.api, client, route.trip_id.clone()),
            |result| match result {
                Ok(details) => RetryableResult::Ok(details),
                Err(error) => RetryableResult::TransientErr {
                    error,
                    override_retry_after: None,
                },
            },
            None,
        )
        .instrument(info_span!("fetch-one-route"))
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to fetch individual route."))?;

        let Some(stations_on_route) = stations_on_route else {
            warn!(
                route_id = %route.route_id,
                route = %route.route,
                "Route did not contain any stations."
            );
            continue;
        };


        // Join with the per-station per-trip timetable data
        // we collected into `bus_trip_to_timetable` earlier.
        let mut stations_with_timetables = Vec::with_capacity(stations_on_route.len());

        for station_on_route in stations_on_route {
            let associated_station_timetable =
                match raw_route_timetables.get(&station_on_route.station_code) {
                    Some(timetable) => timetable,
                    None => {
                        // It's possible that just one station on the route's way
                        // did not return a timetable. In that case, we consider it bad
                        // data and ignore the entire route.
                        error!(
                            route = %route.route,
                            station_code = %station_on_route.station_code,
                            "Did not find a timetable for station on the bus route. \
                            Will ignore the entire route (not fatal)."
                        );
                        continue;
                    }
                };

            stations_with_timetables.push(TripStationWithTimetable {
                station: station_on_route,
                timetable: associated_station_timetable.clone(),
            });
        }


        routes_with_context.push(TripWithStationsAndTimetables {
            captured_at,
            route_details: route,
            stations_on_route_with_timetables: stations_with_timetables,
        });
    }

    // We've processed all the stations and all the routes, including their timetables.
    info!("Finished requesting a snapshot of all stations and routes.");


    let snapshot_time = Utc::now();

    let station_details_snapshot = AllStationsSnapshot::new(snapshot_time, stations_with_bus_trips);
    let route_details_snapshot = AllRoutesSnapshot::new(snapshot_time, routes_with_context);


    // We have the data we need, so it's not time-critical
    // that we save it at this exact moment; let's yield.
    yield_now().await;

    debug!("Saving station and route details to disk.");


    // Save station details.
    let station_details_file_path = station_storage.generate_json_file_path(snapshot_time);

    save_json_to_file(
        &station_details_snapshot,
        &station_details_file_path,
    )
    .wrap_err_with(|| miette!("Failed to save station details snapshot."))?;

    info!(
        file_path = %station_details_file_path.display(),
        "A snapshot of current station details have been saved to disk."
    );


    // Save route details.
    let route_details_file_path = route_storage.generate_json_file_path(snapshot_time);

    save_json_to_file(&route_details_snapshot, &route_details_file_path)
        .wrap_err_with(|| miette!("Failed to save a snapshot of route details."))?;

    info!(
        file_path = %route_details_file_path.display(),
        "A snapshot of current route details have been saved to disk."
    );


    info!("A full snapshot of both route and station details has been successfully saved.");

    Ok(())
}

async fn station_and_route_details_snapshot_loop(
    configuration: LppConfiguration,
    client: Client,
    cancellation_token: CancellationToken,
    run_mode: RunMode,
) -> Result<()> {
    let stations_storage = configuration
        .recording
        .recording_storage_root
        .stations()
        .wrap_err_with(|| miette!("Failed to initialize storage location for station details."))?;

    let route_storage = configuration
        .recording
        .recording_storage_root
        .routes()
        .wrap_err_with(|| miette!("Failed to initialize storage location for route details."))?;


    #[allow(clippy::never_loop)]
    while !cancellation_token.is_cancelled() {
        let time_begin = Instant::now();

        info!("Performing station and route snapshot.");

        make_station_and_route_snapshot(
            &configuration,
            &client,
            &stations_storage,
            &route_storage,
        )
        .await?;

        info!("Station and route snapshot complete.");

        if run_mode == RunMode::Once {
            info!("Run mode is \"once\", exiting.");
            return Ok(());
        }


        // Wait for the configured amount of time
        // until the next snapshot should be captured.
        let time_since_start_of_request = time_begin.elapsed();

        let time_to_wait_until_next_capture = configuration
            .recording
            .full_station_and_timetable_details_request_interval
            .saturating_sub(time_since_start_of_request);

        info!(
            sleep_duration_seconds = time_to_wait_until_next_capture.as_secs(),
            "Snapshot loop will sleep until it's time for the next station snapshot."
        );

        tokio::time::sleep(time_to_wait_until_next_capture).await;
    }

    info!("Station and route snapshotting loop has been cancelled, exiting.");
    Ok(())
}


pub fn initialize_station_and_route_details_snapshot_task(
    config: &LppConfiguration,
    http_client: Client,
    cancellation_token: CancellationToken,
    run_mode: RunMode,
) -> tokio::task::JoinHandle<Result<()>> {
    let station_fetching_span = info_span!("station-details-recorder");
    let station_details_fetching_future = station_and_route_details_snapshot_loop(
        config.clone(),
        http_client,
        cancellation_token,
        run_mode,
    )
    .instrument(station_fetching_span);

    info!("Spawning station details recorder task.");
    tokio::task::spawn(station_details_fetching_future)
}


#[allow(dead_code)]
pub enum RetryableResult<O, E>
where
    E: Error,
{
    Ok(O),
    PermanentErr {
        error: E,
    },
    TransientErr {
        error: E,
        override_retry_after: Option<Duration>,
    },
}

#[derive(Error, Debug)]
pub enum RetryableError {
    #[error("Encountered a permanent error while retrying: {error}")]
    PermamentError { error: miette::Report },

    #[error("Timed out while retrying operation.")]
    TimedOut,
}

pub async fn retryable_async_with_exponential_backoff<C, F, O, P, E, R>(
    future_producer: C,
    future_output_validator: P,
    backoff: Option<ExponentialBackoff<backoff::SystemClock>>,
) -> Result<R, RetryableError>
where
    C: Fn() -> F,
    F: Future<Output = O>,
    P: Fn(O) -> RetryableResult<R, E>,
    E: Diagnostic + Send + Sync + 'static,
{
    let mut exponential_backoff = backoff.unwrap_or_else(|| {
        ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_secs(2))
            .with_randomization_factor(0.1)
            .with_multiplier(2.0)
            .with_max_interval(Duration::from_secs(20))
            .with_max_elapsed_time(Some(Duration::from_secs(60 * 2)))
            .build()
    });

    loop {
        // Generate a future and await it.
        let future_output = future_producer().await;

        // Process the future's output with the user-provided closure.
        // That closure will make a verdict about whether the output is
        // ok, has a transient error, meaning we should retry, or has a permanent error,
        // which should abort retries immediately.
        match future_output_validator(future_output) {
            RetryableResult::Ok(final_value) => return Ok(final_value),
            RetryableResult::PermanentErr { error } => {
                return Err(RetryableError::PermamentError {
                    error: miette::Report::new(error),
                })
            }
            RetryableResult::TransientErr {
                error,
                override_retry_after,
            } => {
                warn!(
                    retry_after = override_retry_after.map(|after| after.as_secs_f64()),
                    transient_error = ?error,
                    "Encountered a transient error, will retry."
                );

                let real_retry_after = match override_retry_after {
                    Some(after) => {
                        exponential_backoff.next_backoff();
                        Some(after)
                    }
                    None => exponential_backoff.next_backoff(),
                };

                if let Some(retry_after) = real_retry_after {
                    tokio::time::sleep(retry_after).await;
                } else {
                    // We've hit the retry limit, abort.
                    return Err(RetryableError::TimedOut);
                }

                continue;
            }
        };
    }
}

/*
#[deprecated]
async fn route_state_fetching_loop(
    configuration: LppConfiguration,
    client: Client,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let route_storage = configuration
        .recording
        .recording_storage_root
        .routes()
        .wrap_err_with(|| miette!("Failed to initialize storage location for route details."))?;


    while !cancellation_token.is_cancelled() {
        let time_begin = Instant::now();
        debug!("Requesting details for all routes from LPP API.");

        let all_routes = retryable_async_with_exponential_backoff(
            || fetch_all_routes(&configuration.api, &client),
            |result| match result {
                Ok(details) => RetryableResult::Ok(details),
                Err(error) => RetryableResult::TransientErr {
                    error,
                    override_retry_after: None,
                },
            },
            None,
        )
        .instrument(info_span!("fetch-all-routes"))
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to fetch all routes."))?;

        info!(
            number_of_routes = all_routes.len(),
            "Fetched all routes, will get stations and timetables for each."
        );

        let mut route_snapshots: Vec<TripWithStationsAndTimetables> =
            Vec::with_capacity(all_routes.len());


        // TODO Merge this and the station details loop - request timetables for all buses on the entire station
        //      and then smartly merge them into a station and route snapshot instead of doing so many requests.

        for route in all_routes {
            info!(
                route_id = %route.route_id,
                route = %route.route,
                "Fetching stations and timetables for route."
            );

            let captured_at = Utc::now();

            let stations_on_route = retryable_async_with_exponential_backoff(
                || fetch_stations_on_route(&configuration.api, &client, route.trip_id.clone()),
                |result| match result {
                    Ok(details) => RetryableResult::Ok(details),
                    Err(error) => RetryableResult::TransientErr {
                        error,
                        override_retry_after: None,
                    },
                },
                None,
            )
            .instrument(info_span!("fetch-one-route"))
            .await
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to fetch individual route."))?;

            let Some(stations_on_route) = stations_on_route else {
                warn!(
                    route_id = %route.route_id,
                    route = %route.route,
                    "Route did not contain any stations."
                );
                continue;
            };


            let mut stations_with_timetables: Vec<TripStationWithTimetable> =
                Vec::with_capacity(stations_on_route.len());

            for station in stations_on_route {
                let mut timetable = retryable_async_with_exponential_backoff(
                    || {
                        fetch_timetable(
                            &configuration.api,
                            &client,
                            &station.station_code,
                            [route.route.to_base_route()],
                            TimetableFetchMode::FullDay,
                        )
                    },
                    |result| match result {
                        Ok(timetable) => RetryableResult::Ok(timetable),
                        Err(error) => RetryableResult::TransientErr {
                            error,
                            override_retry_after: None,
                        },
                    },
                    None,
                )
                .instrument(info_span!("fetch-timetable"))
                .await
                .into_diagnostic()
                .wrap_err_with(|| miette!("Failed to fetch individual timetable."))?;

                if timetable.is_empty() {
                    warn!(
                        station = %station.station_code,
                        full_route = %route.route,
                        "LPP API returned no timetables for station!"
                    );
                    continue;
                } else if timetable.len() > 1 {
                    warn!(
                        station = %station.station_code,
                        full_route = %route.route,
                        "LPP API returned more than one timetable for station!"
                    );
                }

                // PANIC SAFETY: We checked above that it isn't empty.
                let final_timetable = timetable.remove(0);


                trace!(
                    station = %station.station_code,
                    full_route = %route.route,
                    "Got new station + timetable."
                );

                stations_with_timetables.push(TripStationWithTimetable {
                    station,
                    timetable: final_timetable,
                });
            }

            let route_snapshot = TripWithStationsAndTimetables {
                captured_at,
                route_details: route,
                stations_on_route_with_timetables: stations_with_timetables,
            };

            route_snapshots.push(route_snapshot);
        }


        debug!("Saving route details to disk.");
        let final_snapshot_time = Utc::now();
        let snapshot_data = AllRoutesSnapshot {
            captured_at: final_snapshot_time,
            routes: route_snapshots,
        };


        // We have the data we need, so it's not time-critical
        // that we save it at this exact moment; let's yield.
        yield_now().await;


        let file_path = route_storage.generate_json_file_path(final_snapshot_time);
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        save_json_to_file(&snapshot_data, &file_path)
            .wrap_err_with(|| miette!("Failed to save a snapshot of all route details."))?;

        info!(
            file_name = file_name,
            "A snapshot of current route details have been saved to disk."
        );


        // Wait for the configured amount of time
        // until the next snapshot should be captured.
        let time_since_start_of_request = time_begin.elapsed();

        let time_to_wait_until_next_capture = configuration
            .recording
            .route_details_fetching_interval
            .saturating_sub(time_since_start_of_request);

        info!(
            sleep_duration_seconds = time_to_wait_until_next_capture.as_secs(),
            "Snapshot loop will sleep until it's time for the next snapshot of routes."
        );

        tokio::time::sleep(time_to_wait_until_next_capture).await;
    }

    info!("Route details fetching loop has been cancelled, exiting.");
    Ok(())
}


#[deprecated]
pub fn initialize_route_state_recording(
    configuration: &LppConfiguration,
    client: Client,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    let info_fetching_span = info_span!("route-state-recorder");
    let route_state_fetching_future =
        route_state_fetching_loop(configuration.clone(), client, cancellation_token)
            .instrument(info_fetching_span);

    info!("Spawning route state recorder task.");
    tokio::task::spawn(route_state_fetching_future)
}

 */
