/// NewTypes for values from the CUBE API.
///
/// Big thanks to:
/// https://www.worthe-it.co.za/blog/2020-10-31-newtype-pattern-in-rust.html

use console::style;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
extern crate derive_more;

#[derive(Debug, PartialEq)]
pub enum CUBEApiUrlParseErrorReason {
    EndpointVersion,
    Protocol,
}

fn to_styled_str(r: &CUBEApiUrlParseErrorReason) -> String {
    match r {
        CUBEApiUrlParseErrorReason::EndpointVersion => format!(
            "Given URL does not end with \"{}\" (the trailing slash is required)",
            style("/api/v1/").bold()
        ),
        CUBEApiUrlParseErrorReason::Protocol => format!(
            "Given URL does not start with \"{}\" or \"{}\"",
            style("http://").bold(),
            style("https://").bold()
        ),
    }
}

impl Display for CUBEApiUrlParseErrorReason {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", to_styled_str(self))
    }
}

#[derive(Debug, PartialEq)]
pub struct CUBEApiUrlParseError {
    pub value: String,
    pub reason: CUBEApiUrlParseErrorReason,
}

impl Display for CUBEApiUrlParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.reason, style(&self.value).cyan())
    }
}

impl std::error::Error for CUBEApiUrlParseError {}

/// A [CUBEApiUrl] is the base URL for a CUBE, e.g.
/// "https://cube.chrisproject.org/api/v1/"
#[derive(Debug, PartialEq, derive_more::Display, Clone, Serialize, Deserialize)]
pub struct CUBEApiUrl(String);

impl CUBEApiUrl {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::str::FromStr for CUBEApiUrl {
    type Err = CUBEApiUrlParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ss = s.to_string();
        if !(s.starts_with("http://") || s.starts_with("https://")) {
            Err(CUBEApiUrlParseError {
                value: ss,
                reason: CUBEApiUrlParseErrorReason::Protocol,
            })
        } else if !s.ends_with("/api/v1/") {
            Err(CUBEApiUrlParseError {
                value: ss,
                reason: CUBEApiUrlParseErrorReason::EndpointVersion,
            })
        } else {
            Ok(CUBEApiUrl(ss))
        }
    }
}

/// *ChRIS* user's username.
#[derive(Debug, derive_more::FromStr, derive_more::Display, derive_more::Deref, Serialize, Deserialize, PartialEq, Clone)]
pub struct Username(String);

impl Username {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// CUBE user resource URL, e.g. https://cube.chrisproject.org/api/v1/users/3/
#[derive(derive_more::FromStr, derive_more::Display, Serialize, Deserialize)]
pub struct UserUrl(String);

/// CUBE User ID
#[derive(Clone, Copy, Deserialize)]
pub struct UserId(u16);

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn test_username() {
        assert!(Username::from_str("hello").is_ok())
    }

    #[test]
    fn test_parse_url() {
        let bad_protocol = "idk://something";
        assert_eq!(
            CUBEApiUrl::from_str(&bad_protocol),
            Err(CUBEApiUrlParseError {
                value: bad_protocol.to_string(),
                reason: CUBEApiUrlParseErrorReason::Protocol
            })
        );

        let bad_api_vers = vec![
            "https://localhost",
            "http://localhost/",
            "http://localhost/api/v2/",
            "http://localhost/api/v1",
        ];
        for bad_example in bad_api_vers {
            assert_eq!(
                CUBEApiUrl::from_str(&bad_example),
                Err(CUBEApiUrlParseError {
                    value: bad_example.to_string(),
                    reason: CUBEApiUrlParseErrorReason::EndpointVersion
                })
            );
        }

        assert!(CUBEApiUrl::from_str("http://localhost/api/v1/").is_ok());
        assert!(CUBEApiUrl::from_str("https://localhost/api/v1/").is_ok());
    }

    #[test]
    fn test_serialize() -> Result<(), Box<dyn std::error::Error>> {
        let username = Username::from_str("hello")?;
        println!("{}", serde_json::to_string(&username).unwrap());
        Ok(())
    }
}
