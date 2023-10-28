use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSecondsWithFrac};

use crate::api::station_details::StationDetails;

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StationDetailsSnapshot {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,
    pub station_details: Vec<StationDetails>,
}

impl StationDetailsSnapshot {
    pub fn new(timestamp: DateTime<Utc>, station_details: Vec<StationDetails>) -> Self {
        Self {
            captured_at: timestamp,
            station_details,
        }
    }
}
