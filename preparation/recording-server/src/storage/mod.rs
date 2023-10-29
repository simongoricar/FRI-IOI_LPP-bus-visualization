use std::{
    fs,
    io,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use miette::Diagnostic;
use thiserror::Error;


#[derive(Error, Debug, Diagnostic)]
pub enum StorageError {
    #[error("Expected \"{}\" to be a directory.", .path.display())]
    PathIsNotADirectory { path: PathBuf },

    #[error("Encountered other IO error: {0}")]
    OtherIoError(#[from] io::Error),
}

const DATE_TIME_FORMAT: &str = "%Y-%m-%d_%H-%M-%S%.3f+UTC";

fn ensure_directory_exists(path: &Path) -> Result<(), StorageError> {
    if path.exists() && !path.is_dir() {
        return Err(StorageError::PathIsNotADirectory {
            path: path.to_path_buf(),
        });
    } else {
        fs::create_dir_all(path)?;
    };

    Ok(())
}


#[derive(Debug, Clone)]
pub struct StorageRoot {
    base_storage_path: PathBuf,
}

impl StorageRoot {
    pub fn new<P>(base_storage_path: P) -> Result<Self, StorageError>
    where
        P: Into<PathBuf>,
    {
        let base_storage_path: PathBuf = base_storage_path.into();
        ensure_directory_exists(&base_storage_path)?;

        Ok(Self { base_storage_path })
    }

    pub fn path(&self) -> &Path {
        &self.base_storage_path
    }

    pub fn stations(&self) -> Result<StationStorage, StorageError> {
        StationStorage::new(self.base_storage_path.join("stations"))
    }

    pub fn routes(&self) -> Result<RouteStorage, StorageError> {
        RouteStorage::new(self.base_storage_path.join("routes"))
    }

    pub fn arrivals(&self) -> Result<ArrivalStorageRoot, StorageError> {
        ArrivalStorageRoot::new(self.base_storage_path.join("arrival-snapshots"))
    }
}


#[derive(Debug, Clone)]
pub struct StationStorage {
    stations_storage_path: PathBuf,
}

impl StationStorage {
    pub fn new<P>(stations_storage_path: P) -> Result<Self, StorageError>
    where
        P: Into<PathBuf>,
    {
        let stations_storage_path: PathBuf = stations_storage_path.into();
        ensure_directory_exists(&stations_storage_path)?;

        Ok(Self {
            stations_storage_path,
        })
    }

    pub fn directory_path(&self) -> &Path {
        &self.stations_storage_path
    }

    pub fn generate_json_file_path(&self, at_time: DateTime<Utc>) -> PathBuf {
        let formatted_time = at_time.format(DATE_TIME_FORMAT);
        let file_name = format!("station-details_{}.json", formatted_time);

        self.stations_storage_path.join(file_name)
    }
}


#[derive(Debug, Clone)]
pub struct RouteStorage {
    route_storage_root_path: PathBuf,
}

impl RouteStorage {
    pub fn new<P>(route_storage_root_path: P) -> Result<Self, StorageError>
    where
        P: Into<PathBuf>,
    {
        let route_storage_root_path: PathBuf = route_storage_root_path.into();
        ensure_directory_exists(&route_storage_root_path)?;

        Ok(Self {
            route_storage_root_path,
        })
    }

    pub fn directory_path(&self) -> &Path {
        &self.route_storage_root_path
    }

    pub fn generate_json_file_path(&self, at_time: DateTime<Utc>) -> PathBuf {
        let formatted_time = at_time.format(DATE_TIME_FORMAT);
        let file_name = format!("route-details_{}.json", formatted_time);

        self.route_storage_root_path.join(file_name)
    }
}



#[derive(Debug, Clone)]
pub struct ArrivalStorageRoot {
    arrival_storage_root_path: PathBuf,
}

impl ArrivalStorageRoot {
    pub fn new<P>(arrival_storage_root_path: P) -> Result<Self, StorageError>
    where
        P: Into<PathBuf>,
    {
        let arrival_storage_root_path: PathBuf = arrival_storage_root_path.into();
        ensure_directory_exists(&arrival_storage_root_path)?;

        Ok(Self {
            arrival_storage_root_path,
        })
    }

    pub fn directory_path(&self) -> &Path {
        &self.arrival_storage_root_path
    }
}


pub struct ArrivalStorage {
    full_route_name: String,
    arrival_storage_path: PathBuf,
}

impl ArrivalStorage {
    pub fn new<P, N>(arrival_storage_root_path: P, route_name: N) -> Result<Self, StorageError>
    where
        P: Into<PathBuf>,
        N: Into<String>,
    {
        let arrival_storage_root_path: PathBuf = arrival_storage_root_path.into();
        let route_name: String = route_name.into();

        let arrival_storage_path = arrival_storage_root_path.join(&route_name);
        ensure_directory_exists(&arrival_storage_path)?;

        Ok(Self {
            full_route_name: route_name,
            arrival_storage_path,
        })
    }

    pub fn route_name(&self) -> &str {
        &self.full_route_name
    }

    pub fn directory_path(&self) -> &Path {
        &self.arrival_storage_path
    }

    pub fn generate_json_file_path(&self, at_time: DateTime<Utc>) -> PathBuf {
        let formatted_time = at_time.format(DATE_TIME_FORMAT);
        let file_name = format!("arrival_{}.json", formatted_time);

        self.arrival_storage_path.join(file_name)
    }
}
