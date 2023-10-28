use miette::Diagnostic;
use reqwest::StatusCode;
use thiserror::Error;

#[derive(Error, Debug, PartialEq, Eq, Clone)]
pub enum FullUrlConstructionError {
    #[error("failed to join sub-URL onto base: {reason}.")]
    FailedToJoinUrl {
        #[from]
        reason: url::ParseError,
    },
}


#[derive(Error, Debug)]
pub enum LppApiFetchError {
    #[error("URL construction error: {0}")]
    UrlError(#[from] FullUrlConstructionError),

    #[error("Failed to perform request: {0}")]
    RequestError(reqwest::Error),

    /// This can happend when e.g. the `success` field is set to `false` in the JSON response.
    #[error("Request was not successful: {reason}")]
    APIResponseNotSuccessful { reason: String },

    #[error(
        "Received response was malformed (or did the schema change?).{}",
        match reason.as_ref() {
            Some(reason) => reason,
            None => ""
        }
    )]
    APIResponseMalformed { reason: Option<String> },

    #[error("HTTP request failed with client error: {0}")]
    ClientHTTPError(StatusCode),

    #[error("HTTP request failed with server error: {0}")]
    ServerHTTPError(StatusCode),

    #[error("Failed to decode JSON response: {0}")]
    ResponseDecodingError(reqwest::Error),
}

impl LppApiFetchError {
    pub fn malformed_response() -> Self {
        Self::APIResponseMalformed { reason: None }
    }

    pub fn malformed_response_with_reason<S>(reason: S) -> Self
    where
        S: Into<String>,
    {
        Self::APIResponseMalformed {
            reason: Some(reason.into()),
        }
    }
}


#[derive(Error, Debug, Diagnostic)]
#[error("Could not parse timetable: {}", reason)]
pub struct RouteTimetableParseError {
    reason: String,
}

impl RouteTimetableParseError {
    pub fn new<S>(reason: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            reason: reason.into(),
        }
    }
}


#[derive(Error, Debug, Diagnostic)]
#[error("Invalid bus route name: {}", route_name)]
pub struct RouteNameParseError {
    route_name: String,
}

impl RouteNameParseError {
    pub fn new<S>(route_name: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            route_name: route_name.into(),
        }
    }
}
