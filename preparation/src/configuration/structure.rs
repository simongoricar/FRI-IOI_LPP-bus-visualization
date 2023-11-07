use std::{
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use miette::{miette, Context, IntoDiagnostic, Result};
use reqwest::Url;
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use super::{traits::ResolvableConfiguration, utilities::get_default_configuration_file_path};
use crate::storage::StorageRoot;

#[derive(Clone)]
pub struct Configuration {
    pub logging: LoggingConfiguration,
    pub lpp: LppConfiguration,
}

#[derive(Deserialize, Clone)]
pub struct UnresolvedConfiguration {
    logging: UnresolvedLoggingConfiguration,
    lpp: UnresolvedLppConfiguration,
}

impl Configuration {
    pub fn load_from_path<P: AsRef<Path>>(configuration_file_path: P) -> Result<Self> {
        let configuration_file_path = configuration_file_path.as_ref();

        let configuration_file_contents = fs::read_to_string(configuration_file_path)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to read configuration file."))?;

        let unresolved_configuration: UnresolvedConfiguration =
            toml::from_str(&configuration_file_contents)
                .into_diagnostic()
                .wrap_err_with(|| miette!("Failed to parse configuration file as TOML."))?;

        let resolved_configuration = unresolved_configuration
            .resolve()
            .wrap_err_with(|| miette!("Failed to resolve configuration."))?;

        Ok(resolved_configuration)
    }

    pub fn load_from_default_path() -> Result<Self> {
        let default_configuration_file_path = get_default_configuration_file_path()
            .wrap_err_with(|| miette!("Failed to construct default configuration file path."))?;

        Self::load_from_path(default_configuration_file_path)
    }
}

impl ResolvableConfiguration for UnresolvedConfiguration {
    type Resolved = Configuration;

    fn resolve(self) -> Result<Self::Resolved> {
        let logging = self
            .logging
            .resolve()
            .wrap_err_with(|| miette!("Failed to resolve table \"logging\"."))?;

        let lpp = self
            .lpp
            .resolve()
            .wrap_err_with(|| miette!("Failed to resolve table \"lpp\"."))?;

        Ok(Self::Resolved { logging, lpp })
    }
}



#[derive(Deserialize, Clone)]
struct UnresolvedLoggingConfiguration {
    console_output_level_filter: String,
    log_file_output_level_filter: String,
    log_file_output_directory: String,
}

#[derive(Clone)]
pub struct LoggingConfiguration {
    pub console_output_level_filter: String,
    pub log_file_output_level_filter: String,
    pub log_file_output_directory: PathBuf,
}

impl ResolvableConfiguration for UnresolvedLoggingConfiguration {
    type Resolved = LoggingConfiguration;

    fn resolve(self) -> Result<Self::Resolved> {
        // Validate the file and console level filters.
        EnvFilter::try_new(&self.console_output_level_filter)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to parse field `console_output_level_filter`"))?;

        EnvFilter::try_new(&self.log_file_output_level_filter)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to parse field `log_file_output_level_filter`"))?;

        let log_file_output_directory = PathBuf::from(self.log_file_output_directory);

        Ok(Self::Resolved {
            console_output_level_filter: self.console_output_level_filter,
            log_file_output_level_filter: self.log_file_output_level_filter,
            log_file_output_directory,
        })
    }
}

impl LoggingConfiguration {
    pub fn console_output_level_filter(&self) -> EnvFilter {
        // SAFETY: This is safe because we checked the input is valid in `resolve`.
        EnvFilter::try_new(&self.console_output_level_filter).unwrap()
    }

    pub fn log_file_output_level_filter(&self) -> EnvFilter {
        // SAFETY: This is safe because we checked the input is valid in `resolve`.
        EnvFilter::try_new(&self.log_file_output_level_filter).unwrap()
    }
}



#[derive(Deserialize, Clone)]
struct UnresolvedLppConfiguration {
    api: UnresolvedLppApiConfiguration,
    recording: UnresolvedLppRecordingConfiguration,
}

#[derive(Clone)]
pub struct LppConfiguration {
    pub api: LppApiConfiguration,
    pub recording: LppRecordingConfiguration,
}

impl ResolvableConfiguration for UnresolvedLppConfiguration {
    type Resolved = LppConfiguration;

    fn resolve(self) -> Result<Self::Resolved> {
        Ok(Self::Resolved {
            api: self.api.resolve()?,
            recording: self.recording.resolve()?,
        })
    }
}



#[derive(Deserialize, Clone)]
struct UnresolvedLppApiConfiguration {
    lpp_base_api_url: String,
    user_agent: String,
}

#[derive(Clone)]
pub struct LppApiConfiguration {
    pub lpp_base_api_url: Url,
    pub user_agent: String,
}

impl ResolvableConfiguration for UnresolvedLppApiConfiguration {
    type Resolved = LppApiConfiguration;

    fn resolve(self) -> Result<Self::Resolved> {
        let lpp_base_api_url = Url::parse(&self.lpp_base_api_url)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to parse lpp_base_api_url as an URL!"))?;

        Ok(Self::Resolved {
            lpp_base_api_url,
            user_agent: self.user_agent,
        })
    }
}



#[derive(Deserialize, Clone)]
struct UnresolvedLppRecordingConfiguration {
    full_station_and_timetable_details_request_interval: String,
    recording_storage_directory_path: String,
}

#[derive(Clone)]
pub struct LppRecordingConfiguration {
    pub full_station_and_timetable_details_request_interval: Duration,
    pub recording_storage_root: StorageRoot,
}

impl ResolvableConfiguration for UnresolvedLppRecordingConfiguration {
    type Resolved = LppRecordingConfiguration;

    fn resolve(self) -> Result<Self::Resolved> {
        let full_station_and_timetable_details_request_interval =
            humantime::parse_duration(&self.full_station_and_timetable_details_request_interval)
                .into_diagnostic()
                .wrap_err_with(|| {
                    miette!(
                        "Failed to parse duration in field `full_station_and_timetable_details_request_interval`. \
                        Did you include spaces (e.g. `6 hours` instead of `6hours`)?"
                    )
                })?;

        let storage_root = StorageRoot::new(self.recording_storage_directory_path)?;


        Ok(Self::Resolved {
            full_station_and_timetable_details_request_interval,
            recording_storage_root: storage_root,
        })
    }
}
