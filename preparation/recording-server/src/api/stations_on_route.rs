use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use super::errors::{FullUrlConstructionError, LppApiFetchError};
use crate::configuration::structure::LppApiConfiguration;

#[derive(Serialize, Deserialize, Clone)]
struct RawStationsOnRouteResponse {
    pub success: bool,
    pub data: Vec<StationOnRoute>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StationOnRoute {
    /// Unique internal station identifier.
    ///
    /// Example: `3307`.
    ///
    /// LPP documentation: "".
    pub station_int_id: i32,

    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `201011`.
    ///
    /// LPP documentation: "Destination of route (direction)".
    pub station_code: String,

    /// Station name.
    ///
    /// Example: `ŽELEZNA`.
    ///
    /// LPP documentation: "Destination of route (direction)".
    pub name: String,

    /// Stop number (starts at 1 and is incremented for
    /// each next station on the bus route).
    ///
    /// Example: `1`.
    ///
    /// LPP documentation: "Order of stations, 1 is starting station".
    pub order_no: i32,

    /// Geographical latitude of the bus station.
    ///
    /// Example: `46.06103968748721`.
    ///
    /// LPP documentation: "Geographical latitude of station".
    pub latitude: f64,

    /// Geographical longitude of the bus station.
    ///
    /// Longitude: `14.5132960445235`.
    ///
    /// LPP documentation: "Geographical longitude of station".
    pub longitude: f64,
}


fn build_stations_on_route_url<S>(
    api_configuration: &LppApiConfiguration,
    trip_id: S,
) -> Result<Url, FullUrlConstructionError>
where
    S: AsRef<str>,
{
    pub const STATIONS_ON_ROUTE_SUB_URL: &str = "route/stations-on-route";

    let mut url = api_configuration
        .lpp_base_api_url
        .join(STATIONS_ON_ROUTE_SUB_URL)?;

    url.query_pairs_mut()
        .append_pair("trip-id", trip_id.as_ref());

    Ok(url)
}

pub async fn fetch_stations_on_route<S>(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    trip_id: S,
) -> Result<Option<Vec<StationOnRoute>>, LppApiFetchError>
where
    S: AsRef<str>,
{
    let full_url = build_stations_on_route_url(api_configuration, trip_id)?;

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
                (was trying to fetch station details)."
            );
        }

        return Err(LppApiFetchError::ClientHTTPError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerHTTPError(response_status));
    }


    let response_raw_json = response
        .json::<RawStationsOnRouteResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseNotSuccessful {
            reason: String::from("success field is false"),
        });
    }


    if response_raw_json.data.is_empty() {
        Ok(None)
    } else {
        Ok(Some(response_raw_json.data))
    }
}
