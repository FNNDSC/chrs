//! NewTypes for values used by users when interacting with the CUBE API.

use aliri_braid::braid;

#[derive(thiserror::Error, Debug)]
pub enum InvalidCUBEUrl {
    #[error("Given URL does not end with \"/api/v1/\": {0}")]
    EndpointVersion(String),

    #[error("Given URL does not start with \"http://\" or \"https://\": {0}")]
    Protocol(String),
}

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
        assert!(CUBEApiUrl::new(url).is_ok());
    }

    #[rstest]
    #[case("idk://localhost/api/v1/")]
    #[case("localhost/api/v1/")]
    fn test_reject_bad_protocol(#[case] url: &str) {
        assert!(matches!(
            CUBEApiUrl::new(url).unwrap_err(),
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
            CUBEApiUrl::new(url).unwrap_err(),
            InvalidCUBEUrl::EndpointVersion { .. }
        ))
    }
}
