use cancellation_token::CancellationToken;
use clap::Parser;
use cli::{CLIArgs, RunMode};
use logging::initialize_tracing;
use miette::{miette, Context, IntoDiagnostic, Result};
use recorder::initialize_station_and_route_details_snapshot_task;
use reqwest::Client;
use tracing::info;

use crate::configuration::Configuration;

mod api;
mod cancellation_token;
mod cli;
mod configuration;
mod logging;
mod recorder;
mod storage;


pub async fn run_tasks(configuration: &Configuration, run_mode: RunMode) -> Result<()> {
    let http_client = Client::builder()
        .user_agent(&configuration.lpp.api.user_agent)
        .build()
        .unwrap();

    let job_cancellation_token = CancellationToken::new();

    let station_and_route_snapshot_task = initialize_station_and_route_details_snapshot_task(
        &configuration.lpp,
        http_client.clone(),
        job_cancellation_token.clone(),
        run_mode,
    );

    info!("Task spawned.");

    station_and_route_snapshot_task
        .await
        .into_diagnostic()
        .wrap_err_with(|| miette!("Station details recorder task panicked!"))??;

    Ok(())
}


#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = CLIArgs::parse();
    let run_mode = cli_args.run_mode()?;

    let configuration = match &cli_args.config_file_path {
        Some(path) => Configuration::load_from_path(path),
        None => Configuration::load_from_default_path(),
    }
    .wrap_err_with(|| miette!("Failed to load configuration from default path."))?;

    let _guard = initialize_tracing(
        configuration.logging.console_output_level_filter(),
        configuration.logging.log_file_output_level_filter(),
        &configuration.logging.log_file_output_directory,
    )
    .wrap_err_with(|| miette!("Failed to initialize tracing."))?;

    run_tasks(&configuration, run_mode).await?;

    drop(_guard);
    Ok(())
}
