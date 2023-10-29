use std::{
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
use tracing::{debug, error, info, info_span, trace, warn, Instrument};

pub mod formats;

use crate::{
    api::{
        routes::fetch_all_routes,
        station_details::fetch_station_details,
        stations_on_route::fetch_stations_on_route,
        timetable::{fetch_timetable, TimetableFetchMode},
    },
    cancellation_token::CancellationToken,
    configuration::LppConfiguration,
    recorder::formats::{
        AllRoutesSnapshot,
        AllStationsSnapshot,
        RouteWithStationsAndTimetables,
        StationWithTimetable,
    },
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
 * Station details capture
 */

async fn station_details_fetching_loop(
    configuration: LppConfiguration,
    client: Client,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let stations_storage = configuration
        .recording
        .recording_storage_root
        .stations()
        .wrap_err_with(|| miette!("Failed to initialize storage location for station details."))?;


    while !cancellation_token.is_cancelled() {
        let time_begin = Instant::now();
        debug!("Requesting station details from LPP API.");


        let station_details = retryable_async_with_exponential_backoff(
            || fetch_station_details(&configuration.api, &client),
            |result| match result {
                Ok(details) => RetryableResult::Ok(details),
                Err(error) => RetryableResult::TransientErr {
                    error,
                    override_retry_after: None,
                },
            },
            None,
        )
        .instrument(info_span!("fetch-station-details"))
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to fetch station details."))?;

        debug!("Saving station details to disk.");

        let snapshot_time = Utc::now();
        let station_details_snapshot = AllStationsSnapshot::new(snapshot_time, station_details);


        // We have the data we need, so it's not time-critical
        // that we save it at this exact moment; let's yield.
        yield_now().await;


        let file_path = stations_storage.generate_json_file_path(snapshot_time);

        save_json_to_file(&station_details_snapshot, &file_path)
            .wrap_err_with(|| miette!("Failed to save station details snapshot."))?;

        info!(
            file_path = %file_path.display(),
            "A snapshot of current station details have been saved to disk."
        );


        // Wait for the configured amount of time
        // until the next snapshot should be captured.
        let time_since_start_of_request = time_begin.elapsed();

        let time_to_wait_until_next_capture = configuration
            .recording
            .station_details_fetching_interval
            .saturating_sub(time_since_start_of_request);

        info!(
            sleep_duration_seconds = time_to_wait_until_next_capture.as_secs(),
            "Snapshot loop will sleep until it's time for the next station snapshot."
        );

        tokio::time::sleep(time_to_wait_until_next_capture).await;
    }

    info!("Station details fetching loop has been cancelled, exiting.");
    Ok(())
}

pub fn initialize_station_details_recording(
    config: &LppConfiguration,
    http_client: Client,
    cancellation_token: CancellationToken,
) -> tokio::task::JoinHandle<Result<()>> {
    let station_fetching_span = info_span!("station-details-recorder");
    let station_details_fetching_future =
        station_details_fetching_loop(config.clone(), http_client, cancellation_token)
            .instrument(station_fetching_span);

    info!("Spawning station details recorder task.");
    tokio::task::spawn(station_details_fetching_future)
}


/*
 * Route details capture
 */

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


        let mut route_snapshots: Vec<RouteWithStationsAndTimetables> =
            Vec::with_capacity(all_routes.len());

        for route in all_routes {
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


            let mut stations_with_timetables: Vec<StationWithTimetable> =
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

                stations_with_timetables.push(StationWithTimetable {
                    station,
                    timetable: final_timetable,
                });
            }

            let route_snapshot = RouteWithStationsAndTimetables {
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


/*
 * Bus arrival capture
 */


pub fn initialize_arrival_recording() {
    todo!();
}
