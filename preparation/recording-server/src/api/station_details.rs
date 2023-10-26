use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use super::errors::{FullUrlConstructionError, LppApiFetchError};
use crate::configuration::structure::LppApiConfiguration;

#[derive(Serialize, Deserialize, Clone)]
struct RawStationDetailsResponse {
    pub success: bool,
    pub data: Vec<StationDetails>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct StationDetails {
    /// Unique internal station identifier.
    ///
    /// Example: `3307`.
    ///
    /// LPP documentation: "Integer ID of station".
    #[serde(rename = "int_id")]
    pub station_int_id: i32,

    /// Geographical latitude of the bus station.
    ///
    /// Example: `46.06103968748721`.
    ///
    /// LPP documentation: "Geo latitude of station".
    pub latitude: f64,

    /// Geographical longitude of the bus station.
    ///
    /// Example: `14.5132960445235`.
    ///
    /// LPP documentation: "Geo longitude of station".
    pub longitude: f64,

    /// Name of the bus station.
    ///
    /// Example: `ŽELEZNA`.
    ///
    /// LPP documentation: "User friendly name of the station".
    pub name: String,

    /// Unique bus station reference (?) identifier used in other requests.
    ///
    /// Example: `201011`.
    ///
    /// LPP documentation: "Ref ID / station code of the station (ex. 600011)".
    #[serde(rename = "ref_id")]
    pub station_code: String,

    /// A list of all route groups that stop on this bus station.
    /// If `show-subroutes=1` is included in the request, this is separated into
    /// sub-routes, such as 3G, 19B, ...
    ///
    /// **For our requests, we always request subroutes.**
    ///
    /// Example: `["3G", "11B", "12", "12D"]`.
    ///
    /// LPP documentation: "Array of route groups on this station.
    /// This contains only route group numbers (1,2,6...). If show-subroutes=1 is set,
    /// this will also include routes like 19I, 19B... with suffixes".
    #[serde(rename = "route_groups_on_station")]
    pub routes_on_station: Vec<String>,
}



fn build_station_details_url(
    api_configuration: &LppApiConfiguration,
) -> Result<Url, FullUrlConstructionError> {
    pub const STATION_DETAILS_SUB_URL: &str = "station-details";

    let mut url = api_configuration
        .lpp_base_api_url
        .join(STATION_DETAILS_SUB_URL)?;

    url.query_pairs_mut().append_pair("show-subroutes", "1");

    Ok(url)
}


/// Fetches information about all available bus stations.
///
/// LPP API documentation for this request is available
/// at <https://data.lpp.si/doc/#api-Station-station_details>.
pub async fn fetch_station_details(
    api_configuration: &LppApiConfiguration,
    client: &Client,
) -> Result<Vec<StationDetails>, LppApiFetchError> {
    let full_url = build_station_details_url(api_configuration)?;

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

        return Err(LppApiFetchError::ClientError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerError(response_status));
    }


    let response_raw_json = response
        .json::<RawStationDetailsResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseError {
            reason: String::from("success field is false"),
        });
    }

    Ok(response_raw_json.data)
}
