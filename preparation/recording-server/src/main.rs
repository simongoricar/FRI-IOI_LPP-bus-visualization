use logging::initialize_tracing;
use miette::{miette, Context, Result};

use crate::configuration::Configuration;

mod api;
mod cancellation_token;
mod configuration;
mod logging;
mod recorder;
mod storage;


fn main() -> Result<()> {
    let configuration = Configuration::load_from_default_path()
        .wrap_err_with(|| miette!("Failed to load configuration from default path."))?;

    let _guard = initialize_tracing(
        configuration.logging.console_output_level,
        configuration.logging.log_file_output_level,
        &configuration.logging.log_file_output_directory,
    )
    .wrap_err_with(|| miette!("Failed to initialize tracing."))?;

    // TODO

    drop(_guard);
    Ok(())
}
