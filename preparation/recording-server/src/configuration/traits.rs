use miette::Result;

use super::structure::Paths;

pub trait InitializableAfterLoad {
    fn initialize(&mut self) -> Result<()>;
}

pub trait InitializableAfterLoadWithPaths {
    fn initialize_with_paths(&mut self, paths: &Paths) -> Result<()>;
}
