use chrono::{Local, Timelike};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use super::{
    errors::{FullUrlConstructionError, LppApiFetchError, RouteTimetableParseError},
    BaseBusRoute,
    BusRoute,
    BusStationCode,
};
use crate::configuration::LppApiConfiguration;


/*
 * RAW RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableResponse {
    success: bool,
    data: RawTimetableData,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableData {
    /// Concise information about the requested station.
    ///
    /// LPP documentation: "Contains station data".
    station: RawTimetableStationData,

    /// Timetables for requested route groups
    /// (because we can request more than one bus line at once for a station).
    ///
    /// LPP documentation: "Array of timetables for requested route groups".
    route_groups: Vec<RawTimetableRouteGroupsData>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableStationData {
    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `600012`.
    ///
    /// LPP documentation: "Reference ID/ station code of station (6 digits, ex. 600011)".
    ref_id: String,

    /// Station name.
    ///
    /// Example: `Bavarski dvor`.
    ///
    /// LPP documentation: "Name of the station".
    name: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableRouteGroupsData {
    /// Route group number the `routes` are about (without prefix or suffix).
    ///
    /// Example: `3`.
    ///
    /// LPP documentation: "Route group number for the array item
    /// (always non-suffixed, ex. 6 instead of 6B)".
    route_group_number: String,

    /// List of trips in this route group. If `route_group_number` is e.g. "3",
    /// you'd probably expect this list to contain e.g. 3, 3G, N3, N3B, ...
    ///
    /// LPP documentation: "Array of timetables for the specific subroute (ex. 6B)".
    routes: Vec<RawTripTimetable>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTripTimetable {
    /// All arrivals for the given station.
    ///
    /// LPP documentation: "Array of arrivals to this station".
    timetable: Vec<RawTimetableRouteTimetableEntry>,

    /// All stations on this sub-route.
    ///
    /// LPP documentation: "Array of all stations on this subroute".
    stations: Vec<RawStationOnTimetable>,

    /// Bus direction (ending station).
    ///
    /// Example: `RUDNIK`.
    ///
    /// LPP documentation: "Name of the station".
    name: String,

    /// Full route name.
    ///
    /// Example: `LITOSTROJ - Bavarski dvor - RUDNIK`.
    ///
    /// LPP documentation: "Name of the full route/trip (start-destination)".
    parent_name: String,

    /// Bus line name (without a prefix or suffix).
    ///
    /// Example: `3`.
    ///
    /// LPP documentation: "Repeated route group number, non suffixed".
    group_name: String,

    /// Can be an empty string, indicating no prefix.
    ///
    /// Example: `N`.
    ///
    /// LPP documentation: "Letter prefix for the route, if it exists, otherwise empty string".
    route_number_prefix: String,

    /// Can be an empty string, indicating no suffix.
    ///
    /// Example: `B`.
    ///
    /// LPP documentation: "Letter suffix for the route, if it exists, otherwise empty string".
    route_number_suffix: String,

    /// Whether this trip ends in a garage.
    ///
    /// Example: `true`.
    ///
    /// LPP documentation: "true if route ends in garage".
    is_garage: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawTimetableRouteTimetableEntry {
    /// The hour of arrival for this entry.
    ///
    /// Example: `5`.
    ///
    /// LPP documentation: none at all.
    hour: i32,

    /// A list of all arrivals in minutes.
    /// For example, if `hour = 13` and `minutes = [11, 52]`,
    /// interpret that as the bus arriving to this station at
    /// 13:11 and 13:52.
    ///
    /// Example: `[19]`.
    ///
    /// LPP documentation: none at all.
    minutes: Vec<i32>,

    /// Whether this is the current hour. Seems mostly useless.
    ///
    /// Example: `false`.
    ///
    /// LPP documentation: "True if this represents arrivals for current hour.".
    is_current: bool,

    ///
    ///
    /// Example: ``.
    ///
    /// LPP documentation: none at all.
    timestamp: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct RawStationOnTimetable {
    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `201011`.
    ///
    /// LPP documentation: "".
    ref_id: String,

    /// Name of the bus station.
    ///
    /// Example: `ŽELEZNA`.
    ///
    /// LPP documentation: "Name of the station".
    name: String,

    /// Stop number (starts at 1 and is incremented for
    /// each next station on the bus route).
    ///
    /// Example: `1`.
    ///
    /// LPP documentation: "Sequential order number of the station on this route".
    order_no: i32,
}



/*
 * PARSED RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteGroupTimetable {
    /// Base route group name (without a prefix or suffix).
    ///
    /// Example: `3`.
    pub route_group_name: BaseBusRoute,

    /// The base route's specific timetables per "sub-route".
    /// We call these "trip timetables" here because this is
    /// essentially a one-way timetable.
    ///
    /// This means we'll (likely) get timetables for route "3G" and "3B"
    /// whenever `route_group_name` is "3".
    pub trip_timetables: Vec<TripTimetable>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TripTimetable {
    /// Describes the full bus route number
    /// (including any route prefix and/or suffix).
    ///
    /// Example: `3G`
    pub route: BusRoute,

    /// Contains the full trip name.
    ///
    /// Example: `LITOSTROJ - Bavarski dvor - RUDNIK`.
    pub trip_name: String,

    /// Contains a short name for this trip
    /// (usually just the destination part of the `name` field).
    ///
    /// Example: `RUDNIK`.
    pub short_trip_name: String,

    /// Whether this trip ends in a garage.
    ///
    /// Example: `true`.
    pub ends_in_garage: bool,

    /// All departures from this station for the given trip.
    pub timetable: Vec<TimetableEntry>,

    /// All bus stops on this trip.
    pub stations: Vec<StationOnTimetable>,
}

/// An individual entry in the timetable,
/// i.e. when the bus is scheduled to arrive.
///
/// ## Invariants
/// - `1 <= hour <= 24`
/// - `0 <= minute <= 59`
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimetableEntry {
    /// Hour of scheduled arrival.
    pub hour: u8,

    /// Minute of scheduled arrival.
    pub minute: u8,
}

impl TimetableEntry {
    pub fn new(hour: u8, minute: u8) -> Result<Self, RouteTimetableParseError> {
        if hour < 1 {
            return Err(RouteTimetableParseError::new(
                "hour value is smaller than 1!",
            ));
        }
        if hour > 24 {
            return Err(RouteTimetableParseError::new(
                "hour value is larger than 24!",
            ));
        }

        if minute > 59 {
            return Err(RouteTimetableParseError::new(
                "minute value is larger than 59!",
            ));
        }

        Ok(Self { hour, minute })
    }
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StationOnTimetable {
    /// Unique bus station identifier
    /// (useful in other station-related requests).
    ///
    /// Example: `201011`.
    pub station_code: String,

    /// Name of the bus station.
    ///
    /// Example: `ŽELEZNA`.
    pub name: String,

    /// Stop number. Starts at 1 and is incremented for
    /// each next station on the bus route.
    ///
    /// Example: `1`.
    pub stop_number: u32,
}


/*
 * Conversions
 */

impl TryFrom<RawTimetableRouteGroupsData> for RouteGroupTimetable {
    type Error = miette::Report;

    fn try_from(value: RawTimetableRouteGroupsData) -> Result<Self, Self::Error> {
        let route_timetables = value
            .routes
            .into_iter()
            .map(TripTimetable::try_from)
            .collect::<Result<_, _>>()?;

        Ok(Self {
            route_group_name: BaseBusRoute::new_from_str(value.route_group_number)?,
            trip_timetables: route_timetables,
        })
    }
}

impl TryFrom<RawTripTimetable> for TripTimetable {
    type Error = RouteTimetableParseError;

    fn try_from(value: RawTripTimetable) -> Result<Self, Self::Error> {
        let mut timetable_entries = Vec::with_capacity(value.timetable.len());
        for raw_timetable_entry in value.timetable {
            let hour = u8::try_from(raw_timetable_entry.hour).map_err(|_| {
                RouteTimetableParseError::new(format!(
                    "hour value can not fit into u8: {}",
                    raw_timetable_entry.hour,
                ))
            })?;

            for raw_minute_entry in raw_timetable_entry.minutes {
                let minute = u8::try_from(raw_minute_entry).map_err(|_| {
                    RouteTimetableParseError::new(format!(
                        "minute value can not fit into u8: {}",
                        raw_minute_entry,
                    ))
                })?;

                let arrival = TimetableEntry::new(hour, minute)?;
                timetable_entries.push(arrival);
            }
        }

        let group_number = value.group_name.parse::<u32>()
            .map_err(|_| RouteTimetableParseError::new(format!(
                "group_name can not fit into u32 (maybe it has a prefix/suffix and is not a group): {}",
                value.group_name,
            )))?;

        let route = BusRoute::from_components(
            if value.route_number_prefix.is_empty() {
                None
            } else {
                Some(value.route_number_prefix)
            },
            group_number,
            if value.route_number_suffix.is_empty() {
                None
            } else {
                Some(value.route_number_suffix)
            },
            None,
        );

        let stations = value
            .stations
            .into_iter()
            .map(StationOnTimetable::try_from)
            .collect::<Result<_, _>>()?;

        Ok(Self {
            trip_name: value.parent_name,
            short_trip_name: value.name,
            route,
            ends_in_garage: value.is_garage,
            timetable: timetable_entries,
            stations,
        })
    }
}

impl TryFrom<RawStationOnTimetable> for StationOnTimetable {
    type Error = RouteTimetableParseError;

    fn try_from(value: RawStationOnTimetable) -> Result<Self, Self::Error> {
        let stop_number = u32::try_from(value.order_no).map_err(|_| {
            RouteTimetableParseError::new(format!(
                "order_no value can not fit into u32: {}",
                value.order_no,
            ))
        })?;

        Ok(Self {
            station_code: value.ref_id,
            name: value.name,
            stop_number,
        })
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

fn build_timetable_url<I>(
    api_configuration: &LppApiConfiguration,
    station_code: &BusStationCode,
    route_group_numbers: I,
    timetable_mode: &TimetableFetchMode,
) -> Result<Url, FullUrlConstructionError>
where
    I: IntoIterator<Item = BaseBusRoute>,
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


    for route_group_number in route_group_numbers.into_iter() {
        url_query_pairs.append_pair(
            "route-group-number",
            &route_group_number.to_string(),
        );
    }

    drop(url_query_pairs);

    Ok(url)
}


pub async fn fetch_timetable<I>(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    station_code: &BusStationCode,
    route_group_numbers: I,
    timetable_mode: TimetableFetchMode,
) -> Result<Vec<RouteGroupTimetable>, LppApiFetchError>
where
    I: IntoIterator<Item = BaseBusRoute>,
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
        .map(RouteGroupTimetable::try_from)
        .collect::<Result<_, _>>()
        .map_err(|error| LppApiFetchError::malformed_response_with_reason(error.to_string()))?;

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
                &BusStationCode::new("600012"),
                [BaseBusRoute::new_from_str("3").unwrap()],
                &TimetableFetchMode::Manual { next_hours: 12, previous_hours: 12 },
            ).unwrap(),
            Url::parse("https://data.lpp.si/api/station/timetable?station-code=600012&next-hours=12&previous-hours=12&route-group-number=3").unwrap()
        );

        assert_eq!(
            build_timetable_url(
                &api_configuration,
                &BusStationCode::new("600012"),
                [BaseBusRoute::new_from_number(3), BaseBusRoute::new_from_number(18)],
                &TimetableFetchMode::Manual { next_hours: 12, previous_hours: 12 },
            ).unwrap(),
            Url::parse("https://data.lpp.si/api/station/timetable?station-code=600012&next-hours=12&previous-hours=12&route-group-number=3&route-group-number=18").unwrap()
        );
    }
}
