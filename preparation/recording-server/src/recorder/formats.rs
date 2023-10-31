use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, TimestampSecondsWithFrac};

use crate::api::{
    routes::RouteDetails,
    routes_on_station::TripOnStation,
    station_details::StationDetails,
    stations_on_route::StationOnRoute,
    timetable::{RouteGroupTimetable, TimetableEntry, TripTimetable},
    GeographicalLocation,
    StationCode,
};


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllStationsSnapshot {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,
    pub station_details: Vec<StationDetailsWithBusDetailsAndTimetables>,
}

impl AllStationsSnapshot {
    pub fn new(
        timestamp: DateTime<Utc>,
        station_details: Vec<StationDetailsWithBusDetailsAndTimetables>,
    ) -> Self {
        Self {
            captured_at: timestamp,
            station_details,
        }
    }
}



#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StationDetailsWithBusDetailsAndTimetables {
    /// Unique bus station identifier
    /// (useful in other station-related requests).
    ///
    /// Example: `201011`.
    pub station_code: StationCode,

    /// Unique *internal* station identifier.
    /// Unused in other parts of the API.
    ///
    /// Example: `3307`.
    pub internal_station_id: i32,

    /// Name of the bus station.
    ///
    /// Example: `ŽELEZNA`.
    pub name: String,

    /// Geographical location of the bus station.
    pub location: GeographicalLocation,

    /// A list of all trips that stop on this bus station.
    pub trips_on_station: Vec<TripOnStation>,

    pub timetables: Vec<RouteGroupTimetable>,
}

impl StationDetailsWithBusDetailsAndTimetables {
    #[inline]
    pub fn from_station_and_trips(
        station: StationDetails,
        trips: Vec<TripOnStation>,
        timetables: Vec<RouteGroupTimetable>,
    ) -> Self {
        Self {
            station_code: station.station_code,
            internal_station_id: station.internal_station_id,
            name: station.name,
            location: station.location,
            trips_on_station: trips,
            timetables,
        }
    }
}



#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AllRoutesSnapshot {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,

    pub routes: Vec<TripWithStationsAndTimetables>,
}

impl AllRoutesSnapshot {
    #[inline]
    pub fn new(captured_at: DateTime<Utc>, routes: Vec<TripWithStationsAndTimetables>) -> Self {
        Self {
            captured_at,
            routes,
        }
    }
}


#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TripWithStationsAndTimetables {
    #[serde_as(as = "TimestampSecondsWithFrac<String>")]
    pub captured_at: DateTime<Utc>,

    pub route_details: RouteDetails,
    pub stations_on_route_with_timetables: Vec<TripStationWithTimetable>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TripStationWithTimetable {
    pub station: StationOnRoute,
    pub timetable: TripTimetable,
}
