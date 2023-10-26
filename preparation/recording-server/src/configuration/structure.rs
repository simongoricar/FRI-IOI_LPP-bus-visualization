use std::{
    fs,
    path::{Path, PathBuf},
    str::FromStr,
};

use miette::{miette, Context, IntoDiagnostic, Result};
use reqwest::Url;
use serde::Deserialize;
use tracing::Level;

use super::{traits::ResolvableConfiguration, utilities::get_default_configuration_file_path};

#[derive(Clone)]
pub struct Configuration {
    pub logging: LoggingConfiguration,
    pub lpp: LppApiConfiguration,
}

#[derive(Deserialize, Clone)]
pub struct UnresolvedConfiguration {
    logging: UnresolvedLoggingConfiguration,
    lpp: UnresolvedLppApiConfiguration,
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
    console_output_level: String,
    log_file_output_level: String,
    log_file_output_directory: String,
}

#[derive(Clone)]
pub struct LoggingConfiguration {
    pub console_output_level: Level,
    pub log_file_output_level: Level,
    pub log_file_output_directory: PathBuf,
}

impl ResolvableConfiguration for UnresolvedLoggingConfiguration {
    type Resolved = LoggingConfiguration;

    fn resolve(self) -> Result<Self::Resolved> {
        let console_output_level = Level::from_str(&self.console_output_level)
            .into_diagnostic()
            .wrap_err_with(|| {
                miette!("Failed to parse field `console_output_level` as a valid logging level.")
            })?;

        let log_file_output_level = Level::from_str(&self.log_file_output_level)
            .into_diagnostic()
            .wrap_err_with(|| {
                miette!("Failed to parse field `log_file_output_level` as a valid logging level.")
            })?;

        let log_file_output_directory = PathBuf::from(self.log_file_output_directory);

        Ok(Self::Resolved {
            console_output_level,
            log_file_output_level,
            log_file_output_directory,
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
