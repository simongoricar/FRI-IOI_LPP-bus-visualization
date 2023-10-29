use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSecondsWithFrac};

use crate::api::{
    routes::RouteDetails,
    station_details::StationDetails,
    stations_on_route::StationOnRoute,
    timetable::RouteGroupTimetable,
};

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllStationsSnapshot {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,
    pub station_details: Vec<StationDetails>,
}

impl AllStationsSnapshot {
    pub fn new(timestamp: DateTime<Utc>, station_details: Vec<StationDetails>) -> Self {
        Self {
            captured_at: timestamp,
            station_details,
        }
    }
}


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllRoutesSnapshot {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,
    pub routes: Vec<RouteWithStationsAndTimetables>,
}


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RouteWithStationsAndTimetables {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,
    pub route_details: RouteDetails,
    pub stations_on_route_with_timetables: Vec<StationWithTimetable>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StationWithTimetable {
    pub station: StationOnRoute,
    pub timetable: RouteGroupTimetable,
}
