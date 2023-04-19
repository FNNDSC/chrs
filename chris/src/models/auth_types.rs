//! NewTypes for values used by users when first interacting and authenticating with the CUBE API.

use crate::errors::InvalidCUBEUrl;
use aliri_braid::braid;

/// A [CUBEApiUrl] is the base URL for a CUBE, e.g.
/// `https://cube.chrisproject.org/api/v1/`
#[braid(validator, serde)]
pub struct CUBEApiUrl(String);

impl aliri_braid::Validator for CUBEApiUrl {
    type Error = InvalidCUBEUrl;

    fn validate(s: &str) -> Result<(), Self::Error> {
        if !(s.starts_with("http://") || s.starts_with("https://")) {
            Err(InvalidCUBEUrl::Protocol(s.to_string()))
        } else if !s.ends_with("/api/v1/") {
            Err(InvalidCUBEUrl::EndpointVersion(s.to_string()))
        } else {
            Ok(())
        }
    }
}

/// *ChRIS* user's username.
#[braid(serde)]
pub struct Username;

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;

    #[rstest]
    #[case("http://localhost/api/v1/")]
    #[case("http://localhost:8000/api/v1/")]
    #[case("https://cube.chrisproject.org/api/v1/")]
    fn test_parse_url(#[case] url: &str) {
        assert!(CUBEApiUrl::try_from(url).is_ok());
    }

    #[rstest]
    #[case("idk://localhost/api/v1/")]
    #[case("localhost/api/v1/")]
    fn test_reject_bad_protocol(#[case] url: &str) {
        assert!(matches!(
            CUBEApiUrl::try_from(url).unwrap_err(),
            InvalidCUBEUrl::Protocol { .. }
        ))
    }

    #[rstest]
    #[case("http://localhost")]
    #[case("http://localhost/")]
    #[case("http://localhost/api/v2/")]
    #[case("http://localhost/api/v1")]
    fn test_reject_bad_endpoint_version(#[case] url: &str) {
        assert!(matches!(
            CUBEApiUrl::try_from(url).unwrap_err(),
            InvalidCUBEUrl::EndpointVersion { .. }
        ))
    }
}
