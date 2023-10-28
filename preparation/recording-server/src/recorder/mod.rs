use std::{
    fs::OpenOptions,
    path::Path,
    time::{Duration, Instant},
};

use backoff::{backoff::Backoff, ExponentialBackoffBuilder};
use chrono::Utc;
use miette::{miette, Context, IntoDiagnostic, Result};
use reqwest::Client;
use serde::Serialize;
use tokio::task::yield_now;
use tracing::{debug, error, info, info_span, warn, Instrument};

pub mod formats;

use crate::{
    api::station_details::fetch_station_details,
    cancellation_token::CancellationToken,
    configuration::{LppApiConfiguration, LppConfiguration, LppRecordingConfiguration},
    recorder::formats::StationDetailsSnapshot,
};


fn save_json_to_file<S>(data: &S, file_path: &Path) -> Result<()>
where
    S: Serialize,
{
    let mut file = OpenOptions::new()
        .create_new(true)
        .open(file_path)
        .into_diagnostic()
        .wrap_err_with(|| miette!("Failed to open file for writing."))?;

    serde_json::to_writer(file, data);

    Ok(())
}


/*
 * Station details capture
 */

async fn station_details_fetching_loop(
    configuration: &LppConfiguration,
    client: Client,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let mut exponential_backoff = ExponentialBackoffBuilder::new()
        .with_initial_interval(Duration::from_secs(2))
        .with_randomization_factor(0.1)
        .with_multiplier(2.0)
        .with_max_interval(Duration::from_secs(20))
        .with_max_elapsed_time(Some(Duration::from_secs(60 * 2)))
        .build();

    let stations_storage = configuration
        .recording
        .recording_storage_root
        .stations()
        .wrap_err_with(|| miette!("Failed to initialize storage location for station details."))?;


    while !cancellation_token.is_cancelled() {
        let time_begin = Instant::now();
        debug!("Requesting station details from LPP API.");


        let station_details = fetch_station_details(&configuration.api, &client).await;
        let station_details = match station_details {
            Ok(details) => details,
            Err(error) => {
                error!(error = ?error, "Failed to get station details.");

                let time_until_retry = exponential_backoff.next_backoff();
                if let Some(time_until_retry) = time_until_retry {
                    warn!(
                        retrying_after = time_until_retry.as_secs_f64(),
                        "Will retry: failed to request station details."
                    );
                    tokio::time::sleep(time_until_retry);
                } else {
                    error!("Aborting: failed to request station details after many retries.");

                    return Err(miette!(
                        "Failed to request stations details after many retries."
                    ));
                }

                continue;
            }
        };

        debug!("Saving station details to disk.");

        let snapshot_time = Utc::now();
        let station_details_snapshot = StationDetailsSnapshot::new(snapshot_time, station_details);


        // We have the data we need, so it's not time-critical
        // that we save it at this exact moment; let's yield.
        yield_now().await;


        let file_path = stations_storage.generate_json_file_path(snapshot_time);
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        save_json_to_file(
            &station_details_snapshot,
            &stations_storage.generate_json_file_path(snapshot_time),
        )
        .wrap_err_with(|| miette!("Failed to save station details snapshot."))?;

        info!(
            file_name = file_name,
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
        station_details_fetching_loop(config, http_client, cancellation_token)
            .instrument(station_fetching_span);

    tokio::task::spawn(station_details_fetching_future)
}


/*
 * Route details capture
 */

async fn route_state_fetching_loop(config: &LppRecordingConfiguration) {
    todo!();
}


pub fn initialize_route_state_recording(config: &LppConfiguration) -> tokio::task::JoinHandle<()> {
    let info_fetching_span = info_span!("route-state-recorder");
    let route_state_fetching_future =
        route_state_fetching_loop(config).instrument(info_fetching_span);

    tokio::task::spawn(route_state_fetching_future)
}


/*
 * Bus arrival capture
 */


pub fn initialize_arrival_recording() {
    todo!();
}
