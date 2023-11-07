use miette::Result;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use url::Url;

use super::{
    errors::{FullUrlConstructionError, LppApiFetchError},
    BusRoute,
    RouteId,
    StationCode,
    TripId,
};
use crate::configuration::LppApiConfiguration;



/*
 * RAW RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone)]
struct RawRoutesOnStationResponse {
    success: bool,
    data: Vec<RawRouteOnStation>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RawRouteOnStation {
    /// Unique route identifier. This identifies all directions of
    /// a route, e.g. bus 3G going to Bežigrad and 3G going to Grosuplje have the same `route_id`.
    ///
    /// LPP documentation: "ID of route"
    ///
    /// Example: `A48D5D5E-1A10-4616-86BE-65B059E0A371`
    route_id: String,

    /// Unique trip identifier. This uniquely identifies a single direction
    /// of some route, e.g. bus 3G going specifically from Grosuplje to Bežigrad.
    ///
    /// LPP documentation: "ID of specific route"
    ///
    /// Example: `BD96D5A0-76D3-4B3B-94E1-069A3A0B18DD`
    trip_id: String,

    /// Describes the bus number (can be prefixed or suffixed).
    ///
    /// LPP documentation: "Number + suffix letter of route group"
    ///
    /// Example: `3G`
    route_number: String,

    /// Contains a short naming for this route (well, trip).
    ///
    /// LPP documentation: "Name of route destination"
    ///
    /// Example: `BEŽIGRAD`
    route_name: Option<String>,

    /// Contains the full route (well, trip) name.
    ///
    /// LPP documentation: "Full name of route (start - destination)"
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    route_group_name: String,

    /// Specifies whether this route (well, trip) will end in the garage.
    ///
    /// LPP documentation: "Does this route go to depot".
    ///
    /// Example: `true`
    is_garage: bool,
}


/*
 * PARSED RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TripOnStation {
    /// Unique route identifier. This identifies all directions of
    /// a route, e.g. bus 3G going to Bežigrad and 3G going to Grosuplje have the same `route_id`.
    ///
    /// Example: `A48D5D5E-1A10-4616-86BE-65B059E0A371`
    pub route_id: RouteId,

    /// Unique trip identifier. This uniquely identifies a single direction
    /// of some route, e.g. bus 3G going specifically from Grosuplje to Bežigrad.
    ///
    /// Example: `BD96D5A0-76D3-4B3B-94E1-069A3A0B18DD`
    pub trip_id: TripId,

    /// Describes the bus number (can be prefixed or suffixed).
    ///
    /// Example: `3G`
    pub route: BusRoute,

    /// Contains a short naming for this route (well, trip).
    ///
    /// Example: `BEŽIGRAD`
    pub short_trip_name: Option<String>,

    /// Contains the full route (well, trip) name.
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    pub trip_name: String,

    /// Specifies whether this route (well, trip) will end in the garage.
    ///
    /// Example: `true`
    pub ends_in_garage: bool,
}

impl TryFrom<RawRouteOnStation> for TripOnStation {
    type Error = miette::Report;

    fn try_from(value: RawRouteOnStation) -> std::result::Result<Self, Self::Error> {
        let route = BusRoute::from_route_name(value.route_number)?;

        Ok(Self {
            route_id: RouteId::new(value.route_id),
            trip_id: TripId::new(value.trip_id),
            route,
            short_trip_name: value.route_name,
            trip_name: value.route_group_name,
            ends_in_garage: value.is_garage,
        })
    }
}


/*
 * FETCHING
 */


fn build_routes_on_station_url(
    api_configuration: &LppApiConfiguration,
    station_code: &StationCode,
) -> Result<Url, FullUrlConstructionError> {
    pub const ROUTES_ON_STATION_SUB_URL: &str = "station/routes-on-station";

    let mut url = api_configuration
        .lpp_base_api_url
        .join(ROUTES_ON_STATION_SUB_URL)?;

    url.query_pairs_mut()
        .append_pair("station-code", station_code.as_ref());

    Ok(url)
}


pub async fn fetch_routes_on_station(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    station_code: &StationCode,
) -> Result<Vec<TripOnStation>, LppApiFetchError> {
    let full_url = build_routes_on_station_url(api_configuration, station_code)?;

    debug!(
        full_url = %full_url,
        station_code = %station_code,
        "Will fetch routes for station from the LPP API."
    );


    let response = client
        .get(full_url)
        .send()
        .await
        .map_err(LppApiFetchError::RequestError)?;

    let response_status = response.status();
    if response_status.is_client_error() {
        if response_status.eq(&StatusCode::TOO_MANY_REQUESTS) {
            warn!(
                "LPP API is rate-limiting us! Got 429 Too Many Requests \
                (was trying to fetch routes on station)."
            );
        }

        return Err(LppApiFetchError::ClientHTTPError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerHTTPError(response_status));
    }


    let response_raw_json = response
        .json::<RawRoutesOnStationResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseNotSuccessful {
            reason: String::from("success field is false"),
        });
    }


    let parsed_trips = response_raw_json
        .data
        .into_iter()
        .map(TripOnStation::try_from)
        .collect::<Result<_>>()
        .map_err(|error| LppApiFetchError::malformed_response_with_reason(error.to_string()))?;

    Ok(parsed_trips)
}

// TODO
