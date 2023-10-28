use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

use super::errors::RouteNameParseError;

/// Represents a location on the Earth in the
/// [geographical coordinate system](https://en.wikipedia.org/wiki/Geographic_coordinate_system).
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
pub struct GeographicalLocation {
    /// Geographical latitude.
    ///
    /// Example: `46.06103968748721`.
    pub latitude: f64,

    /// Geographical longitude.
    ///
    /// Example: `14.5132960445235`.
    pub longitude: f64,
}

impl GeographicalLocation {
    #[inline]
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
        }
    }
}



/// A newtype representing a bus station ID. This is called
/// a *code* to differentiate it from the *internal ID* the API also exposes.
///
/// ## Usage and `ref_id` vs `int_id`
/// Such an ID is usually returned as `ref_id` (and not `int_id`)
/// in API responses from LPP and can be used in subsequent requests
/// where the station ID is required. The `int_id` fields seem to
/// only be internal IDs that are unusued in other parts of their API.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BusStationCode(String);

impl BusStationCode {
    #[inline]
    pub fn new<S>(id: S) -> Self
    where
        S: Into<String>,
    {
        Self(id.into())
    }
}

impl From<String> for BusStationCode {
    fn from(value: String) -> Self {
        Self(value)
    }
}



/// Represents a full bus route name
/// (including a potential prefix and/or suffix).
///
/// Example: `11B`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct BusRoute {
    pub prefix: Option<String>,
    pub base_route_number: u32,
    pub suffix: Option<String>,
}

impl BusRoute {
    fn components_from_route_name(
        full_route_name: String,
    ) -> Result<(Option<String>, u32, Option<String>), RouteNameParseError> {
        if full_route_name.is_empty() {
            return Err(RouteNameParseError::new(full_route_name));
        }

        let mut route_name_graphemes = full_route_name.graphemes(true);

        let first_grapheme = route_name_graphemes
            .next()
            .ok_or_else(|| RouteNameParseError::new(&full_route_name))?
            .to_string();


        let prefix = if first_grapheme.parse::<u8>().is_err() {
            // The prefix exists, i.e. the first grapheme is not a number.
            Some(first_grapheme.to_uppercase().to_string())
        } else {
            None
        };

        let last_grapheme = route_name_graphemes
            .last()
            .ok_or_else(|| RouteNameParseError::new(&full_route_name))?
            .to_string();

        let suffix = if last_grapheme.parse::<u8>().is_err() {
            // The suffix exists, i.e. the last grapheme is not a number.
            Some(last_grapheme.to_uppercase().to_string())
        } else {
            None
        };


        let base_route_number = {
            let mut modified_route_name = if let Some(prefix) = prefix.as_ref() {
                full_route_name
                    .strip_prefix(prefix)
                    .ok_or_else(|| RouteNameParseError::new(&full_route_name))?
            } else {
                full_route_name.as_str()
            };

            modified_route_name = if let Some(suffix) = suffix.as_ref() {
                modified_route_name
                    .strip_suffix(suffix)
                    .ok_or_else(|| RouteNameParseError::new(&full_route_name))?
            } else {
                modified_route_name
            };


            modified_route_name
                .parse::<u32>()
                .map_err(|_| RouteNameParseError::new(&full_route_name))?
        };

        Ok((prefix, base_route_number, suffix))
    }

    pub fn from_route_name<S>(route_name: S) -> Result<Self, RouteNameParseError>
    where
        S: Into<String>,
    {
        let (prefix, base_route_number, suffix) =
            Self::components_from_route_name(route_name.into())?;

        Ok(Self {
            prefix,
            base_route_number,
            suffix,
        })
    }

    #[inline]
    pub fn from_components(
        prefix: Option<String>,
        base_route_number: u32,
        suffix: Option<String>,
    ) -> Self {
        Self {
            prefix,
            base_route_number,
            suffix,
        }
    }
}

impl TryFrom<String> for BusRoute {
    type Error = RouteNameParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_route_name(value)
    }
}

impl ToString for BusRoute {
    fn to_string(&self) -> String {
        format!(
            "{prefix}{base}{suffix}",
            prefix = match self.prefix.as_ref() {
                Some(prefix) => prefix,
                None => "",
            },
            base = self.base_route_number,
            suffix = match self.suffix.as_ref() {
                Some(suffix) => suffix,
                None => "",
            }
        )
    }
}



/// Represents a bus route name
/// *without a prefix or suffix*, i.e. the "base" route.
///
/// Example: `11`.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BaseBusRoute(String);

impl BaseBusRoute {
    #[inline]
    pub fn new<S>(base_bus_route_name: S) -> Self
    where
        S: Into<String>,
    {
        Self(base_bus_route_name.into())
    }
}

impl From<String> for BaseBusRoute {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl ToString for BaseBusRoute {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}



#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct RouteId(String);

impl RouteId {
    #[inline]
    pub fn new<S>(route_id: S) -> Self
    where
        S: Into<String>,
    {
        Self(route_id.into())
    }
}

impl From<String> for RouteId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}


#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct VehicleId(String);

impl VehicleId {
    #[inline]
    pub fn new<S>(vehicle_id: S) -> Self
    where
        S: Into<String>,
    {
        Self(vehicle_id.into())
    }
}

impl From<String> for VehicleId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}



#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct TripId(String);

impl TripId {
    #[inline]
    pub fn new<S>(trip_id: S) -> Self
    where
        S: Into<String>,
    {
        Self(trip_id.into())
    }
}

impl From<String> for TripId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}



#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bus_route_correctly() {
        assert_eq!(
            BusRoute::from_route_name("19").unwrap(),
            BusRoute::from_components(None, 19, None),
        );

        assert_eq!(
            BusRoute::from_route_name("3G").unwrap(),
            BusRoute::from_components(None, 3, Some("G".to_string())),
        );

        assert_eq!(
            BusRoute::from_route_name("N1").unwrap(),
            BusRoute::from_components(Some("N".to_string()), 1, None),
        );

        assert_eq!(
            BusRoute::from_route_name("N3B").unwrap(),
            BusRoute::from_components(Some("N".to_string()), 3, Some("B".to_string())),
        );
    }
}
