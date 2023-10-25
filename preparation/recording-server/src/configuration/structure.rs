use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use miette::{miette, Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};

use super::{
    traits::{InitializableAfterLoad, InitializableAfterLoadWithPaths},
    utilities::{get_default_configuration_file_path, replace_placeholders_in_path},
};

#[derive(Serialize, Deserialize, Clone)]
pub struct Configuration {
    // TODO
}

impl Configuration {
    pub fn load_from_path<P: AsRef<Path>>(configuration_file_path: P) -> Result<Self> {
        let configuration_file_path = configuration_file_path.as_ref();

        let configuration_file_contents = fs::read_to_string(configuration_file_path)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to read configuration file."))?;

        let mut configuration: Self = toml::from_str(&configuration_file_contents)
            .into_diagnostic()
            .wrap_err_with(|| miette!("Failed to parse configuration file as TOML."))?;

        configuration.initialize()?;

        Ok(configuration)
    }

    pub fn load_from_default_path() -> Result<Self> {
        let default_configuration_file_path = get_default_configuration_file_path()
            .wrap_err_with(|| miette!("Failed to construct default configuration file path."))?;

        Self::load_from_path(default_configuration_file_path)
    }
}

impl InitializableAfterLoad for Configuration {
    fn initialize(&mut self) -> Result<()> {
        todo!();
    }
}
