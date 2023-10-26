use chrono::{Local, Timelike};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use super::errors::{FullUrlConstructionError, LppApiFetchError};
use crate::configuration::structure::LppApiConfiguration;


/*
 * Raw timetable structs
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableResponse {
    pub success: bool,
    pub data: RawTimetableData,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableData {
    /// Concise information about the requested station.
    ///
    /// LPP documentation: "Contains station data".
    pub station: RawTimetableStationData,

    /// Timetables for requested route groups
    /// (because we can request more than one bus line at once for a station).
    ///
    /// LPP documentation: "Array of timetables for requested route groups".
    pub route_groups: Vec<RawTimetableRouteGroupsData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableStationData {
    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `600012`.
    ///
    /// LPP documentation: "Reference ID/ station code of station (6 digits, ex. 600011)".
    #[serde(rename = "ref_id")]
    pub station_code: String,

    /// Station name.
    ///
    /// Example: `Bavarski dvor`.
    ///
    /// LPP documentation: "Name of the station".
    pub name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableRouteGroupsData {
    /// Route group number the `routes` are about (without prefix or suffix).
    ///
    /// Example: `3`.
    ///
    /// LPP documentation: "Route group number for the array item
    /// (always non-suffixed, ex. 6 instead of 6B)".
    pub route_group_number: String,

    /// List of trips in this route group. If `route_group_number` is e.g. "3",
    /// you'd probably expect this list to contain e.g. 3, 3G, N3, N3B, ...
    ///
    /// LPP documentation: "Array of timetables for the specific subroute (ex. 6B)".
    pub routes: Vec<RawTimetableRoute>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableRoute {
    /// All arrivals for the given station.
    ///
    /// LPP documentation: "Array of arrivals to this station".
    pub timetable: Vec<RawTimetableRouteTimetableEntry>,

    /// All stations on this sub-route.
    ///
    /// LPP documentation: "Array of all stations on this subroute".
    pub stations: Vec<RawTimetableRouteStationEntry>,

    /// Bus direction (ending station).
    ///
    /// Example: `RUDNIK`.
    ///
    /// LPP documentation: "Name of the station".
    pub name: String,

    /// Full route name.
    ///
    /// Example: `LITOSTROJ - Bavarski dvor - RUDNIK`.
    ///
    /// LPP documentation: "Name of the full route/trip (start-destination)".
    pub parent_name: String,

    /// Bus line name (without a prefix or suffix).
    ///
    /// Example: `3`.
    ///
    /// LPP documentation: "Repeated route group number, non suffixed".
    pub group_name: String,

    /// Can be an empty string, indicating no prefix.
    ///
    /// Example: `N`.
    ///
    /// LPP documentation: "Letter prefix for the route, if it exists, otherwise empty string".
    pub route_number_prefix: String,

    /// Can be an empty string, indicating no suffix.
    ///
    /// Example: `B`.
    ///
    /// LPP documentation: "Letter suffix for the route, if it exists, otherwise empty string".
    pub route_number_suffix: String,

    /// Whether this trip ends in a garage.
    ///
    /// Example: `true`.
    ///
    /// LPP documentation: "true if route ends in garage".
    pub is_garage: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableRouteTimetableEntry {
    /// The hour of arrival for this entry.
    ///
    /// Example: `5`.
    ///
    /// LPP documentation: none at all.
    pub hour: i32,

    /// A list of all arrivals in minutes.
    /// For example, if `hour = 13` and `minutes = [11, 52]`,
    /// interpret that as the bus arriving to this station at
    /// 13:11 and 13:52.
    ///
    /// Example: `[19]`.
    ///
    /// LPP documentation: none at all.
    pub minutes: Vec<i32>,

    /// Whether this is the current hour. Seems mostly useless.
    ///
    /// Example: `false`.
    ///
    /// LPP documentation: "True if this represents arrivals for current hour.".
    pub is_current: bool,

    ///
    ///
    /// Example: ``.
    ///
    /// LPP documentation: none at all.
    pub timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableRouteStationEntry {
    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `201011`.
    ///
    /// LPP documentation: "".
    #[serde(rename = "ref_id")]
    pub station_code: String,

    /// Name of the bus station.
    ///
    /// Example: `ŽELEZNA`.
    ///
    /// LPP documentation: "Name of the station".
    pub name: String,

    /// Stop number (starts at 1 and is incremented for
    /// each next station on the bus route).
    ///
    /// Example: `1`.
    ///
    /// LPP documentation: "Sequential order number of the station on this route".
    pub order_no: i32,
}

/*
 * Parsed timetable structs
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteGroupTimetable {
    /// Route group number (without prefix or suffix).
    /// `trip_timetables` includes sub-routes of
    ///
    /// Example: `3`.
    pub route_group_name: String,

    pub route_timetables: Vec<RouteTimetable>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteTimetable {
    /// Full route name.
    ///
    /// Example: `LITOSTROJ - Bavarski dvor - RUDNIK`.
    pub route_name: String,

    /// Short route name - bus direction (ending station).
    ///
    /// Example: `RUDNIK`.
    pub short_route_name: String,

    /// Bus line name (without a prefix or suffix).
    ///
    /// Example: `3`.
    pub route_group_name: String,

    /// Can be an empty string, indicating no prefix.
    ///
    /// Example: `N`.
    pub route_number_prefix: String,

    /// Can be an empty string, indicating no suffix.
    ///
    /// Example: `B`.
    pub route_number_suffix: String,

    /// Whether this trip ends in a garage.
    ///
    /// Example: `true`.
    pub is_garage: bool,

    /// All arrivals of this sub-route for the given station.
    pub timetable: Vec<RouteTimetableEntry>,

    /// All stations on this sub-route.
    pub stations: Vec<RouteStationEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteTimetableEntry {
    pub hour: i32,
    pub minute: i32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteStationEntry {
    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `201011`.
    pub station_code: String,

    /// Name of the bus station.
    ///
    /// Example: `ŽELEZNA`.
    pub name: String,

    /// Stop number (starts at 1 and is incremented for
    /// each next station on the bus route).
    ///
    /// Example: `1`.
    pub order_no: i32,
}


/*
 * Conversions
 */

impl From<RawTimetableRouteGroupsData> for RouteGroupTimetable {
    fn from(value: RawTimetableRouteGroupsData) -> Self {
        let route_timetables = value
            .routes
            .into_iter()
            .map(|raw_route| RouteTimetable::from(raw_route))
            .collect();

        Self {
            route_group_name: value.route_group_number,
            route_timetables,
        }
    }
}

impl From<RawTimetableRoute> for RouteTimetable {
    fn from(value: RawTimetableRoute) -> Self {
        let mut timetable_entries = Vec::with_capacity(value.timetable.len());
        for raw_timetable_entry in value.timetable {
            for raw_minute_entry in raw_timetable_entry.minutes {
                timetable_entries.push(RouteTimetableEntry {
                    hour: raw_timetable_entry.hour,
                    minute: raw_minute_entry,
                });
            }
        }


        let stations = value
            .stations
            .into_iter()
            .map(RouteStationEntry::from)
            .collect::<Vec<_>>();

        Self {
            route_name: value.parent_name,
            short_route_name: value.name,
            route_group_name: value.group_name,
            route_number_prefix: value.route_number_prefix,
            route_number_suffix: value.route_number_suffix,
            is_garage: value.is_garage,
            timetable: timetable_entries,
            stations,
        }
    }
}

impl From<RawTimetableRouteStationEntry> for RouteStationEntry {
    fn from(value: RawTimetableRouteStationEntry) -> Self {
        Self {
            station_code: value.station_code,
            name: value.name,
            order_no: value.order_no,
        }
    }
}


/*
 * Fetching
 */


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TimetableFetchMode {
    /// Automatically capture timetables for the entire day.
    FullDay,

    /// Capture timetables for up to `previous_hours` before
    /// and `next_hours` after fetching.
    Manual {
        next_hours: u32,
        previous_hours: u32,
    },
}

fn build_timetable_url<S, N, I>(
    api_configuration: &LppApiConfiguration,
    station_code: S,
    route_group_numbers: I,
    timetable_mode: &TimetableFetchMode,
) -> Result<Url, FullUrlConstructionError>
where
    S: AsRef<str>,
    N: AsRef<str>,
    I: IntoIterator<Item = N>,
{
    pub const TIMETABLE_SUB_URL: &str = "station/timetable";

    let mut url = api_configuration.lpp_base_api_url.join(TIMETABLE_SUB_URL)?;
    let mut url_query_pairs = url.query_pairs_mut();

    url_query_pairs.append_pair("station-code", station_code.as_ref());


    let (next_hours, previous_hours) = match timetable_mode {
        TimetableFetchMode::FullDay => {
            // Automatically set next and previous to capture entire day.
            let local_time_now = Local::now();
            let current_hour = local_time_now.hour();

            let next_hours = current_hour;
            let previous_hours = 24u32.saturating_sub(current_hour);

            (next_hours, previous_hours)
        }
        TimetableFetchMode::Manual {
            next_hours,
            previous_hours,
        } => (*next_hours, *previous_hours),
    };

    url_query_pairs.append_pair("next-hours", &next_hours.to_string());
    url_query_pairs.append_pair("previous-hours", &previous_hours.to_string());


    for station_code in route_group_numbers.into_iter() {
        url_query_pairs.append_pair("route-group-number", station_code.as_ref());
    }

    drop(url_query_pairs);

    Ok(url)
}


pub async fn fetch_timetable<S, N, I>(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    station_code: S,
    route_group_numbers: I,
    timetable_mode: TimetableFetchMode,
) -> Result<Vec<RouteGroupTimetable>, LppApiFetchError>
where
    S: AsRef<str>,
    N: AsRef<str>,
    I: IntoIterator<Item = N>,
{
    let full_url = build_timetable_url(
        api_configuration,
        station_code,
        route_group_numbers,
        &timetable_mode,
    )?;

    let response = client
        .get(full_url)
        .header("User-Agent", &api_configuration.user_agent)
        .send()
        .await
        .map_err(LppApiFetchError::RequestError)?;


    let response_status = response.status();
    if response_status.is_client_error() {
        if response_status.eq(&StatusCode::TOO_MANY_REQUESTS) {
            warn!(
                "LPP API is rate-limiting us! Got 429 Too Many Requests \
                (was trying to fetch timetables)."
            );
        }

        return Err(LppApiFetchError::ClientHTTPError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerHTTPError(response_status));
    }


    let response_raw_json = response
        .json::<RawTimetableResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseNotSuccessful {
            reason: String::from("success field is false"),
        });
    }


    let route_group_timetables = response_raw_json
        .data
        .route_groups
        .into_iter()
        .map(RouteGroupTimetable::from)
        .collect();

    Ok(route_group_timetables)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn properly_build_timetable_url() {
        let api_configuration = LppApiConfiguration {
            lpp_base_api_url: Url::parse("https://data.lpp.si/api/").unwrap(),
            user_agent: String::from("visualization-recorder / 1.0.0"),
        };


        assert_eq!(
            build_timetable_url(
                &api_configuration,
                "600012",
                ["3"],
                &TimetableFetchMode::Manual { next_hours: 12, previous_hours: 12 },
            ).unwrap(),
            Url::parse("https://data.lpp.si/api/station/timetable?station-code=600012&next-hours=12&previous-hours=12&route-group-number=3").unwrap()
        );

        assert_eq!(
            build_timetable_url(
                &api_configuration,
                "600012",
                ["3", "18"],
                &TimetableFetchMode::Manual { next_hours: 12, previous_hours: 12 },
            ).unwrap(),
            Url::parse("https://data.lpp.si/api/station/timetable?station-code=600012&next-hours=12&previous-hours=12&route-group-number=3&route-group-number=18").unwrap()
        );
    }
}
