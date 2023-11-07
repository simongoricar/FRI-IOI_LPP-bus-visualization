#![allow(dead_code)]

use miette::{miette, Result};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use super::{
    errors::{FullUrlConstructionError, LppApiFetchError},
    BusRoute,
    GeographicalLocation,
    RouteId,
    StationCode,
    VehicleId,
};
use crate::configuration::LppApiConfiguration;

/*
 * RAW RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone)]
struct RawArrivalsOnRouteResponse {
    success: bool,
    data: Vec<RawStationArrivalDetails>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RawStationArrivalDetails {
    /// Unique internal station identifier.
    ///
    /// Example: `3307`.
    ///
    /// LPP documentation: "Integer ID of station".
    station_int_id: i32,

    /// Station name.
    ///
    /// Example: `ŽELEZNA`.
    ///
    /// LPP documentation: "Destination of route (direction)".
    name: String,

    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `201011`.
    ///
    /// LPP documentation: "Destination of route (direction)".
    station_code: String,

    /// Stop number (starts at 1 and is incremented for
    /// each next station on the bus route).
    ///
    /// Example: `1`.
    ///
    /// LPP documentation: "Order of stations, 1 is starting station".
    order_no: i32,

    /// Geographical latitude of the bus station.
    ///
    /// Example: `46.06103968748721`.
    ///
    /// LPP documentation: "Geographical latitude of station".
    latitude: f64,

    /// Geographical longitude of the bus station.
    ///
    /// Longitude: `14.5132960445235`.
    ///
    /// LPP documentation: "Geographical longitude of station".
    longitude: f64,

    /// Live arrival data (be it tabletime-based
    /// or from live GPS estimations - type of prediction is tagged).
    ///
    /// LPP documentation: "Array of arrivals for this station.
    /// Only arrivals of busses driving on this route.
    /// Arrivals are ordered by ascending eta_min field.".
    arrivals: Vec<RawArrivalData>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RawArrivalData {
    /// Unique route identifier belonging to this trip.
    ///
    /// LPP documentation: "ID of the route (parent of trip ID)".
    route_id: String,

    /// Internal LPP vehicle ID (the rest of the vehicle-related API is
    /// locked behind authentication).
    ///
    /// LPP documentation: "ID of the vehicle".
    vehicle_id: String,

    /// Type of prediction in `eta_min`:
    /// - `0` means the field is a live estimation,
    /// - `1` means the field is just how the bus is supposed to arrive based on the timetable,
    /// - `2` means the bus is currently arriving to the station and
    /// - `3` means the bus will not stop at this station due to a detour.
    ///
    /// LPP documentation: "A type of arrival: (0 - predicted,
    /// 1 - scheduled, 2 - approaching station (prihod), 3 - detour (obvoz))"-
    r#type: i32,

    /// Estimated time of arrival in minutes.
    ///
    /// LPP documentation: "Estimated time of arrival in minutes".
    eta_min: i32,

    /// Name of the route.
    ///
    /// Example: `1`.
    ///
    /// LPP documentation: "Name of route (1, 6B, N5...)".
    route_name: String,

    /// Full trip name.
    ///
    /// Example: `MESTNI LOG - VIŽMARJE`.
    ///
    /// LPP documentation: "Name of this trip, in format - ".
    trip_name: String,

    /// - `0` if on normal route
    /// - `1` if heading to garage
    ///
    /// LPP documentation: "0 if normal route, 1 if vehicle is headed to garage".
    depot: i32,
}


/*
 * PARSED RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct StationArrivalDetails {
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

    /// Station name.
    ///
    /// Example: `ŽELEZNA`.
    pub name: String,

    /// Stop number. Starts at 1 and is incremented for
    /// each next station on the bus route.
    ///
    /// Example: `1`.
    pub stop_number: u32,

    /// Geographical location of the bus station.
    pub location: GeographicalLocation,

    /// Arrival data. This can be be tabletime-based
    /// or from live GPS estimations (though we don't have access
    /// to the actual location). The prediction type is annotated.
    pub arrivals: Vec<ArrivalData>,
}


impl TryFrom<RawStationArrivalDetails> for StationArrivalDetails {
    type Error = miette::Report;

    fn try_from(value: RawStationArrivalDetails) -> std::result::Result<Self, Self::Error> {
        let stop_number = u32::try_from(value.order_no)
            .map_err(|_| miette!("Invalid value of field `order_no`: not u32"))?;

        let arrivals = value
            .arrivals
            .into_iter()
            .map(ArrivalData::try_from)
            .collect::<Result<Vec<_>>>()?;

        Ok(Self {
            station_code: StationCode::new(value.station_code),
            internal_station_id: value.station_int_id,
            name: value.name,
            stop_number,
            location: GeographicalLocation::new(value.latitude, value.longitude),
            arrivals,
        })
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ArrivalEstimation {
    /// Estimated time of arrival is derived from the bus location
    /// (i.e. the bus is on its way).
    LocationBased { eta_in_minutes: u32 },

    /// Estimated time of arrival is derived from the timetable
    /// (i.e. the bus has not departed).
    TimetableBased { eta_in_minutes: u32 },

    /// The bus is currently arriving to this bus station.
    /// This variant is usually reported by the api
    /// instead of [`Self::LocationBased`]`{ eta_in_minutes: 0}`.
    CurrentlyArrivingToStation,

    /// The bus will not stop on this station due to a detour.
    OnDetour,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ArrivalData {
    /// Unique route identifier belonging to this trip.
    pub route_id: RouteId,

    /// Internal LPP vehicle ID.
    ///
    /// The rest of the vehicle-related API is
    /// locked behind authentication.
    pub vehicle_id: VehicleId,

    /// Arrival estimation.
    pub arrival_estimation: ArrivalEstimation,

    /// Describes the full bus route number
    /// (including any route prefix and/or suffix).
    ///
    /// Example: `1`, `3G`.
    pub route: BusRoute,

    /// Full trip name.
    ///
    /// Example: `MESTNI LOG - VIŽMARJE`.
    pub trip_name: String,

    /// Whether the bus is on a route that will, at some point,
    /// head to the garage.
    pub heading_to_garage: bool,
}


impl TryFrom<RawArrivalData> for ArrivalData {
    type Error = miette::Report;

    fn try_from(value: RawArrivalData) -> Result<Self, Self::Error> {
        let eta_in_minutes = u32::try_from(value.eta_min)
            .map_err(|_| miette!("Invalid value for `eta_min` field: does not fit in u32"))?;

        let estimation = match value.r#type {
            0 => ArrivalEstimation::LocationBased { eta_in_minutes },
            1 => ArrivalEstimation::TimetableBased { eta_in_minutes },
            2 => ArrivalEstimation::CurrentlyArrivingToStation,
            3 => ArrivalEstimation::OnDetour,
            unknown_value => {
                return Err(miette!(
                    "Unrecognized `type` field value: expected 0/1/2/3, got {}",
                    unknown_value
                ))
            }
        };

        let route = BusRoute::from_route_name(value.route_name)?;

        let heading_to_garage = match value.depot {
            0 => false,
            1 => true,
            unknown_value => {
                return Err(miette!(
                    "Unrecognized `depot` field value: expected 0 or 1, got {}",
                    unknown_value
                ))
            }
        };

        Ok(Self {
            route_id: RouteId::new(value.route_id),
            vehicle_id: VehicleId::new(value.vehicle_id),
            arrival_estimation: estimation,
            route,
            trip_name: value.trip_name,
            heading_to_garage,
        })
    }
}


/*
 * FETCHING
 */

fn build_arrivals_on_route_url<T>(
    api_configuration: &LppApiConfiguration,
    trip_id: T,
) -> Result<Url, FullUrlConstructionError>
where
    T: AsRef<str>,
{
    pub const ARRIVALS_ON_ROUTE_SUB_URL: &str = "route/arrivals-on-route";

    let mut url = api_configuration
        .lpp_base_api_url
        .join(ARRIVALS_ON_ROUTE_SUB_URL)?;

    url.query_pairs_mut()
        .append_pair("trip-id", trip_id.as_ref());

    Ok(url)
}



pub async fn fetch_arrivals_on_route<T>(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    trip_id: T,
) -> Result<Vec<StationArrivalDetails>, LppApiFetchError>
where
    T: AsRef<str>,
{
    let full_url = build_arrivals_on_route_url(api_configuration, trip_id)?;

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
                (was trying to fetch arrivals on route)."
            );
        }

        return Err(LppApiFetchError::ClientHTTPError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerHTTPError(response_status));
    }


    let response_raw_json = response
        .json::<RawArrivalsOnRouteResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseNotSuccessful {
            reason: String::from("success field is false"),
        });
    }


    let parsed_details = response_raw_json
        .data
        .into_iter()
        .map(StationArrivalDetails::try_from)
        .collect::<Result<Vec<_>>>()
        .map_err(|error| LppApiFetchError::malformed_response_with_reason(error.to_string()))?;

    Ok(parsed_details)
}
