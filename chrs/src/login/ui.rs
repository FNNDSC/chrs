use aliri_braid::braid;
use std::borrow::Cow;
use chris::FeedResponse;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("Invalid ChRIS_ui URL")]
pub struct InvalidUiUrl;

aliri_braid::from_infallible!(InvalidUiUrl);

/// ChRIS_ui URL (without trailing slash)
#[braid(serde, normalizer)]
pub struct UiUrl;

impl UiUrl {
    pub fn feed_url_of(&self, feed: &FeedResponse) -> String {
        let t = if feed.public { "public" } else { "private" };
        format!("{}/feeds/{}?type={}", &self.as_str(), feed.id.0, t)
    }
}

impl aliri_braid::Validator for UiUrl {
    type Error = InvalidUiUrl;
    fn validate(s: &str) -> Result<(), Self::Error> {
        if s.starts_with("http://") || s.starts_with("https://") {
            Ok(())
        } else {
            Err(InvalidUiUrl)
        }
    }
}

impl aliri_braid::Normalizer for UiUrl {
    fn normalize(s: &str) -> Result<Cow<str>, Self::Error> {
        let ds = s.find("//").ok_or(InvalidUiUrl)? + 2;
        if let Some(i) = s[ds..].find('/') {
            let proto = &s[0..ds];
            Ok(Cow::Borrowed(&s[0..(proto.len() + i)]))
        } else {
            Ok(Cow::Borrowed(s))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use std::str::FromStr;

    #[rstest]
    #[case("https://app.chrisproject.org", "https://app.chrisproject.org")]
    #[case("https://app.chrisproject.org/", "https://app.chrisproject.org")]
    #[case("https://app.chrisproject.org/feeds", "https://app.chrisproject.org")]
    #[case("https://app.chrisproject.org/feeds/", "https://app.chrisproject.org")]
    fn test_ui_url_normalizer(#[case] given: &str, #[case] expected: &str) {
        assert_eq!(UiUrl::from_str(given).unwrap().as_str(), expected)
    }
    #[rstest]
    fn test_ui_url_invalid() {
        assert!(UiUrl::from_str("app.chrisproject.org").is_err())
    }
}
