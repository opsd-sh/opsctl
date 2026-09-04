//! Links from CLI workflows into authenticated website pages.
//!
//! These links contain only ordinary public identifiers. Authentication stays
//! in the browser's HttpOnly session cookie; CLI API credentials are never
//! copied into URLs or passed to the website.

use std::sync::LazyLock;

use url::Url;

static PRODUCTION_WEBSITE_URL: LazyLock<WebsiteUrl> = LazyLock::new(|| {
    WebsiteUrl(
        Url::parse("https://www.opsd.sh/").expect("hard-coded production website URL is valid"),
    )
});

#[derive(Debug, thiserror::Error)]
pub(crate) enum WebsiteError {
    #[error("invalid website URL: {0}")]
    InvalidUrl(&'static str),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct WebsiteUrl(Url);

impl WebsiteUrl {
    pub(crate) fn production() -> Self {
        (*PRODUCTION_WEBSITE_URL).clone()
    }

    pub(crate) fn from_override(url: Url) -> Result<Self, WebsiteError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(WebsiteError::InvalidUrl("scheme must be HTTP or HTTPS"));
        }
        if url.host().is_none() {
            return Err(WebsiteError::InvalidUrl("host is required"));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(WebsiteError::InvalidUrl("credentials are not permitted"));
        }
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            return Err(WebsiteError::InvalidUrl(
                "path, query, and fragment must be empty",
            ));
        }

        Ok(Self(url))
    }

    fn billing_setup_url(&self, business_id: &str) -> Url {
        let mut url = self
            .0
            .join("billing/setup")
            .expect("validated website URL accepts the billing setup path");
        url.query_pairs_mut()
            .append_pair("business_id", business_id);
        url
    }
}

/// Opens billing setup in the user's browser and prints a manual fallback.
pub(crate) fn open_billing_setup(website_url: &WebsiteUrl, business_id: &str) {
    let setup_url = website_url.billing_setup_url(business_id);

    println!("Open this URL to set up billing:");
    println!("{setup_url}");

    if let Err(error) = webbrowser::open(setup_url.as_str()) {
        eprintln!("warning: could not open a browser: {error}");
    }
}

#[cfg(test)]
mod tests {
    use super::{WebsiteError, WebsiteUrl};
    use url::Url;

    #[test]
    fn production_billing_setup_url_contains_only_the_business_id() {
        let website = WebsiteUrl::production();

        assert_eq!(
            website
                .billing_setup_url("22222222-2222-4222-8222-222222222222")
                .as_str(),
            "https://www.opsd.sh/billing/setup?business_id=22222222-2222-4222-8222-222222222222"
        );
    }

    #[test]
    fn website_urls_reject_credentials_and_application_paths() {
        assert!(matches!(
            WebsiteUrl::from_override(Url::parse("https://user@example.com/").unwrap()),
            Err(WebsiteError::InvalidUrl(_))
        ));
        assert!(matches!(
            WebsiteUrl::from_override(Url::parse("https://www.opsd.sh/account").unwrap()),
            Err(WebsiteError::InvalidUrl(_))
        ));
    }
}
