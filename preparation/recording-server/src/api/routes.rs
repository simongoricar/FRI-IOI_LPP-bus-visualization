use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use super::errors::{FullUrlConstructionError, LppApiFetchError};
use crate::configuration::structure::LppApiConfiguration;

#[derive(Serialize, Deserialize, Clone)]
struct RawRoutesResponse {
    pub success: bool,

    /// Per-trip details for all routes.
    pub data: Vec<RouteDetails>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RouteDetails {
    /// Unique route identifier. This identifies all directions of
    /// a route, e.g. bus 3G going to Bežigrad and 3G going to Grosuplje have the same `route_id`.
    ///
    /// LPP documentation: "ID of general route"
    ///
    /// Example: `A48D5D5E-1A10-4616-86BE-65B059E0A371`
    pub route_id: String,

    /// Unique trip identifier. This uniquely identifies a single direction
    /// of some route, e.g. bus 3G going specifically from Grosuplje to Bežigrad.
    ///
    /// LPP documentation: "ID of specific route"
    ///
    /// Example: `BD96D5A0-76D3-4B3B-94E1-069A3A0B18DD`
    pub trip_id: String,

    /// Unique internal trip identifier. Not used in public-facing API.
    ///
    /// LPP documentation: "Integer ID of route"
    ///
    /// Example: `3085`
    pub trip_int_id: i32,

    /// Describes the bus number (can have a one-letter suffix).
    ///
    /// LPP documentation: "Route group number with optional letter suffix if it exists"
    ///
    /// Example: `3G`
    pub route_number: String,

    /// Contains the full route (well, trip) name.
    ///
    /// LPP documentation: "Full route name"
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    pub route_name: String,

    /// Contains a short naming for this route (well, trip).
    ///
    /// LPP documentation: "Destination of route (direction)"
    ///
    /// Example: `BEŽIGRAD`
    pub short_route_name: String,
}


#[derive(Serialize, Deserialize, Clone)]
struct RawRouteWithShapeResponse {
    pub success: bool,

    /// A single route has more than a single trip,
    /// and these details are basically per-trip.
    pub data: Vec<RouteDetailsWithShape>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RouteDetailsWithShape {
    /// Unique route identifier. This identifies all directions of
    /// a route, e.g. bus 3G going to Bežigrad and 3G going to Grosuplje have the same `route_id`.
    ///
    /// LPP documentation: "ID of general route"
    ///
    /// Example: `A48D5D5E-1A10-4616-86BE-65B059E0A371`
    pub route_id: String,

    /// Unique trip identifier. This uniquely identifies a single direction
    /// of some route, e.g. bus 3G going specifically from Grosuplje to Bežigrad.
    ///
    /// LPP documentation: "ID of specific route"
    ///
    /// Example: `BD96D5A0-76D3-4B3B-94E1-069A3A0B18DD`
    pub trip_id: String,

    /// Unique internal trip identifier. Not used in public-facing API.
    ///
    /// LPP documentation: "Integer ID of route"
    ///
    /// Example: `3085`
    pub trip_int_id: i32,

    /// Describes the bus number (can have a one-letter suffix).
    ///
    /// LPP documentation: "Route group number with optional letter suffix if it exists"
    ///
    /// Example: `3G`
    pub route_number: String,

    /// Contains the full route (well, trip) name.
    ///
    /// LPP documentation: "Full route name"
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    pub route_name: String,

    /// Contains a short naming for this route (well, trip).
    ///
    /// LPP documentation: "Destination of route (direction)"
    ///
    /// Example: `BEŽIGRAD`
    pub short_route_name: String,

    #[serde(rename = "geojson_shape")]
    pub route_shape: RouteShape,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RouteShape {
    /// GeoJSON type, in this case this should be "LineString".
    pub r#type: String,

    /// Set of points along which the bus travels.
    pub coordinates: Vec<[f64; 2]>,

    /// Bounding box coordinates of the entire route.
    #[serde(rename = "bbox")]
    pub bounding_box: [f64; 4],
}

impl RouteShape {
    pub fn validate_type(&self) -> Result<(), ()> {
        match self.r#type.eq("LineString") {
            true => Ok(()),
            false => Err(()),
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
enum RouteRequestType {
    AllRoutes,
    SingleRoute { route_id: String, with_shape: bool },
}

fn build_routes_url(
    api_configuration: &LppApiConfiguration,
    request_type: RouteRequestType,
) -> Result<Url, FullUrlConstructionError> {
    pub const ROUTES_SUB_URL: &str = "route/routes";

    let mut url = api_configuration.lpp_base_api_url.join(ROUTES_SUB_URL)?;

    if let RouteRequestType::SingleRoute {
        route_id,
        with_shape,
    } = request_type
    {
        url.query_pairs_mut().append_pair("route-id", &route_id);

        if with_shape {
            url.query_pairs_mut().append_pair("shape", "1");
        }
    }

    Ok(url)
}

pub async fn fetch_all_routes(
    api_configuration: &LppApiConfiguration,
    client: &Client,
) -> Result<Vec<RouteDetails>, LppApiFetchError> {
    let full_url = build_routes_url(api_configuration, RouteRequestType::AllRoutes)?;

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
                (was trying to fetch all routes)."
            );
        }

        return Err(LppApiFetchError::ClientError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerError(response_status));
    }


    let response_raw_json = response
        .json::<RawRoutesResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseError {
            reason: String::from("success field is false"),
        });
    }

    Ok(response_raw_json.data)
}


pub async fn fetch_single_route_with_shape<S>(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    route_id: S,
) -> Result<Vec<RouteDetailsWithShape>, LppApiFetchError>
where
    S: Into<String>,
{
    let full_url = build_routes_url(
        api_configuration,
        RouteRequestType::SingleRoute {
            route_id: route_id.into(),
            with_shape: true,
        },
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
                (was trying to fetch route with shape)."
            );
        }

        return Err(LppApiFetchError::ClientError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerError(response_status));
    }


    let response_raw_json = response
        .json::<RawRouteWithShapeResponse>()
        .await
        .map_err(LppApiFetchError::ResponseDecodingError)?;

    if !response_raw_json.success {
        return Err(LppApiFetchError::APIResponseError {
            reason: String::from("success field is false"),
        });
    }


    for route_details in &response_raw_json.data {
        route_details
            .route_shape
            .validate_type()
            .map_err(|_| LppApiFetchError::APIResponseError {
                reason: format!(
                    "Expected geojson_shape.type to be LineString, got {}!",
                    route_details.route_shape.r#type
                ),
            });
    }

    Ok(response_raw_json.data)
}
