use miette::miette;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};
use url::Url;

use super::{
    errors::{FullUrlConstructionError, LppApiFetchError},
    BusRoute,
    RouteId,
    TripId,
};
use crate::configuration::LppApiConfiguration;

/*
 * RAW RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone)]
struct RawRoutesResponse {
    success: bool,

    /// Per-trip details for all routes.
    data: Vec<RawRouteDetails>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct RawRouteDetails {
    /// Unique route identifier. This identifies all directions of
    /// a route, e.g. bus 3G going to Bežigrad and 3G going to Grosuplje have the same `route_id`.
    ///
    /// LPP documentation: "ID of general route"
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

    /// Unique internal trip identifier. Not used in public-facing API.
    ///
    /// LPP documentation: "Integer ID of route"
    ///
    /// Example: `3085`
    trip_int_id: i32,

    /// Describes the bus number (can have a one-letter suffix).
    ///
    /// LPP documentation: "Route group number with optional letter suffix if it exists"
    ///
    /// Example: `3G`
    route_number: String,

    /// Contains the full route (well, trip) name.
    ///
    /// LPP documentation: "Full route name"
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    route_name: String,

    /// Contains a short naming for this route (well, trip).
    ///
    /// LPP documentation: "Destination of route (direction)"
    ///
    /// Example: `BEŽIGRAD`
    short_route_name: String,
}


#[derive(Serialize, Deserialize, Clone)]
struct RawRouteWithShapeResponse {
    success: bool,

    /// A single route has more than a single trip,
    /// and these details are basically per-trip.
    data: Vec<RawRouteDetailsWithShape>,
}

#[derive(Serialize, Deserialize, Clone)]
struct RawRouteDetailsWithShape {
    /// Unique route identifier. This identifies all directions of
    /// a route, e.g. bus 3G going to Bežigrad and 3G going to Grosuplje have the same `route_id`.
    ///
    /// LPP documentation: "ID of general route"
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

    /// Unique internal trip identifier. Not used in public-facing API.
    ///
    /// LPP documentation: "Integer ID of route"
    ///
    /// Example: `3085`
    trip_int_id: i32,

    /// Describes the bus number (can have a one-letter suffix).
    ///
    /// LPP documentation: "Route group number with optional letter suffix if it exists"
    ///
    /// Example: `3G`
    route_number: String,

    /// Contains the full route (well, trip) name.
    ///
    /// LPP documentation: "Full route name"
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    route_name: String,

    /// Contains a short naming for this route (well, trip).
    ///
    /// LPP documentation: "Destination of route (direction)"
    ///
    /// Example: `BEŽIGRAD`
    short_route_name: String,

    geojson_shape: RawGeoJSONShape,
}

#[derive(Serialize, Deserialize, Clone)]
struct RawGeoJSONShape {
    r#type: String,
    coordinates: Vec<[f64; 2]>,
    bbox: [f64; 4],
}


/*
 * PARSED RESPONSE SCHEMAS
 */

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteDetails {
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

    /// Unique internal trip identifier. Not used in the public-facing API.
    ///
    /// Example: `3085`
    pub internal_trip_id: i32,

    /// Describes the full bus route number
    /// (including any route prefix and/or suffix).
    ///
    /// Example: `3G`
    pub route: BusRoute,

    /// Contains the full trip name.
    ///
    /// Example: `Adamičev spomenik - GROSUPLJE - BEŽIGRAD`
    pub name: String,

    /// Contains a short name for this trip
    /// (usually just the destination part of the `name` field).
    ///
    /// Example: `BEŽIGRAD`
    pub short_name: String,

    /// A GEOJson value contaning the route the bus takes.
    pub route_shape: Option<RouteGeoJsonShape>,
}

impl TryFrom<RawRouteDetails> for RouteDetails {
    type Error = miette::Report;

    fn try_from(value: RawRouteDetails) -> Result<Self, Self::Error> {
        let route = BusRoute::from_route_name(value.route_number)?;

        Ok(Self {
            route_id: RouteId::new(value.route_id),
            trip_id: TripId::new(value.trip_id),
            internal_trip_id: value.trip_int_id,
            route,
            name: value.route_name,
            short_name: value.short_route_name,
            route_shape: None,
        })
    }
}

impl TryFrom<RawRouteDetailsWithShape> for RouteDetails {
    type Error = miette::Report;

    fn try_from(value: RawRouteDetailsWithShape) -> Result<Self, Self::Error> {
        let route = BusRoute::from_route_name(value.route_number)?;
        let route_shape = RouteGeoJsonShape::try_from(value.geojson_shape)?;

        Ok(Self {
            route_id: RouteId::new(value.route_id),
            trip_id: TripId::new(value.trip_id),
            internal_trip_id: value.trip_int_id,
            route,
            name: value.route_name,
            short_name: value.short_route_name,
            route_shape: Some(route_shape),
        })
    }
}


/// GeoJSON LineString data representing the path the bus takes.
///
/// Specification: <https://datatracker.ietf.org/doc/html/rfc7946#appendix-A.2>.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RouteGeoJsonShape {
    /// Set of points along which the bus travels.
    ///
    /// The value pair is made up of the longitude and latitude (in that order).
    pub path_coordinates: Vec<[f64; 2]>,

    /// Bounding box coordinates of the entire route.
    ///
    /// Values are:
    /// - minimum longitude,
    /// - minimum latitude,
    /// - maximum longitude and
    /// - maximum latitude.
    ///
    /// Specification: <https://datatracker.ietf.org/doc/html/rfc7946#section-5>.
    pub bounding_box: [f64; 4],
}

impl TryFrom<RawGeoJSONShape> for RouteGeoJsonShape {
    type Error = miette::Report;

    fn try_from(value: RawGeoJSONShape) -> Result<Self, Self::Error> {
        if value.r#type.to_lowercase().ne("linestring") {
            return Err(miette!(
                "Invalid GeoJSON shape type, expected LineString!"
            ));
        }

        Ok(Self {
            path_coordinates: value.coordinates,
            bounding_box: value.bbox,
        })
    }
}


/*
 * FETCHING
 */

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

    debug!(
        full_url = %full_url,
        "Will fetch all routes from the LPP API."
    );

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

        return Err(LppApiFetchError::ClientHTTPError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerHTTPError(response_status));
    }


    let response_raw_json = response
        .json::<RawRoutesResponse>()
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
        .map(RouteDetails::try_from)
        .collect::<Result<_, _>>()
        .map_err(|error| LppApiFetchError::malformed_response_with_reason(error.to_string()))?;

    Ok(parsed_details)
}


pub async fn fetch_single_route_with_shape<S>(
    api_configuration: &LppApiConfiguration,
    client: &Client,
    route_id: S,
) -> Result<Vec<RouteDetails>, LppApiFetchError>
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

        return Err(LppApiFetchError::ClientHTTPError(response_status));
    } else if response_status.is_server_error() {
        return Err(LppApiFetchError::ServerHTTPError(response_status));
    }


    let response_raw_json = response
        .json::<RawRouteWithShapeResponse>()
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
        .map(RouteDetails::try_from)
        .collect::<Result<_, _>>()
        .map_err(|error| LppApiFetchError::malformed_response_with_reason(error.to_string()))?;

    Ok(parsed_details)
}
