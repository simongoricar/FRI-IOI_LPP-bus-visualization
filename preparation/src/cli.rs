use std::path::PathBuf;

use clap::Parser;
use miette::{miette, Result};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum RunMode {
    Once,
    Perpetual,
}

#[derive(Parser, Debug, Clone)]
pub struct CLIArgs {
    #[arg(
        long = "config-file-path",
        global = true,
        help = "File path of the configuration file. If unspecified, \
                this defaults to ./data/configuration.toml relative to the current directory."
    )]
    pub config_file_path: Option<PathBuf>,

    #[arg(
        long = "run-mode",
        help = "Timetable/station recording mode: \"once\" downloads today's data and exits, \
                \"perpetual\" keeps downloading it as long as configured (24 hours by default)."
    )]
    pub run_mode: Option<String>,
}

impl CLIArgs {
    pub fn run_mode(&self) -> Result<RunMode> {
        match &self.run_mode {
            Some(run_mode) => match run_mode.to_lowercase().as_str() {
                "once" => Ok(RunMode::Once),
                "perpetual" => Ok(RunMode::Perpetual),
                invalid_mode => Err(miette!(
                    "Invalid run mode: {} (expected once/perpetual).",
                    invalid_mode
                )),
            },
            None => Ok(RunMode::Once),
        }
    }
}
