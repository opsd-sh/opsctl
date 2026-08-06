use std::{
    env,
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::LazyLock,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use url::Url;

const CLIENT_ID: &str = "opsctl";
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const TOKEN_TYPE_HINT: &str = "access_token";
const MAX_DEVICE_NAME_CHARACTERS: usize = 80;
const SLOW_DOWN_INCREMENT: Duration = Duration::from_secs(5);
static PRODUCTION_SERVER_URL: LazyLock<ServerUrl> = LazyLock::new(|| {
    ServerUrl(
        Url::parse("https://api.opsd.sh/").expect("hard-coded production server URL is valid"),
    )
});

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthError {
    #[error("invalid server URL: {0}")]
    InvalidServerUrl(&'static str),
    #[error("could not determine the user configuration directory")]
    ConfigDirectoryUnavailable,
    #[error("failed to access CLI credentials at `{path}`: {source}")]
    CredentialIo {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("saved CLI credentials at `{path}` are invalid")]
    InvalidCredentials { path: PathBuf },
    #[error("saved CLI credentials at `{path}` must not be accessible by other users")]
    InsecureCredentialPermissions { path: PathBuf },
    #[error("not logged in; run `opsctl auth login`")]
    NotLoggedIn,
    #[error("the saved login is for {saved}; run `opsctl auth login` for {requested}")]
    ServerMismatch { saved: String, requested: String },
    #[error("the saved login has expired; run `opsctl auth login`")]
    CredentialsExpired,
    #[error("OAuth request failed: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("OAuth endpoint `{endpoint}` returned HTTP {status}")]
    UnexpectedResponse {
        endpoint: &'static str,
        status: StatusCode,
    },
    #[error("OAuth request was rejected: {0}")]
    Protocol(String),
    #[error("the device authorization expired; run `opsctl auth login` again")]
    DeviceAuthorizationExpired,
    #[error("the OAuth server returned an unsupported token type")]
    UnsupportedTokenType,
    #[error("the system clock is before the Unix epoch")]
    InvalidSystemTime,
}

pub(crate) struct AccessToken(String);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ServerUrl(Url);

impl ServerUrl {
    pub(crate) fn production() -> Self {
        (*PRODUCTION_SERVER_URL).clone()
    }

    pub(crate) fn from_override(url: Url) -> Result<Self, AuthError> {
        if !matches!(url.scheme(), "http" | "https") {
            return Err(AuthError::InvalidServerUrl("scheme must be HTTP or HTTPS"));
        }
        if url.host().is_none() {
            return Err(AuthError::InvalidServerUrl("host is required"));
        }
        if !url.username().is_empty() || url.password().is_some() {
            return Err(AuthError::InvalidServerUrl("credentials are not permitted"));
        }
        if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
            return Err(AuthError::InvalidServerUrl(
                "path, query, and fragment must be empty",
            ));
        }

        Ok(Self(url))
    }

    pub(crate) fn public_api_base_url(&self) -> Url {
        self.join("v1/")
    }

    fn device_authorization_url(&self) -> Url {
        self.join("oauth/device/authorization")
    }

    fn token_url(&self) -> Url {
        self.join("oauth/token")
    }

    fn revocation_url(&self) -> Url {
        self.join("oauth/revoke")
    }

    fn join(&self, path: &str) -> Url {
        self.0
            .join(path)
            .expect("validated server URL accepts an application path")
    }

    fn as_url(&self) -> &Url {
        &self.0
    }
}

impl std::fmt::Display for ServerUrl {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl AccessToken {
    pub(crate) fn into_inner(self) -> String {
        self.0
    }
}

impl std::fmt::Debug for AccessToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("AccessToken([REDACTED])")
    }
}

#[derive(Deserialize)]
struct DeviceAuthorizationResponse {
    device_code: String,
    user_code: String,
    verification_uri: Url,
    verification_uri_complete: Url,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    token_type: String,
    expires_in: u64,
}

#[derive(Deserialize)]
struct OAuthErrorResponse {
    error: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PollAction {
    Pending,
    SlowDown,
    Expired,
}

#[derive(Deserialize, Serialize)]
struct StoredCredentials {
    server_url: Url,
    access_token: String,
    expires_at: u64,
}

pub(crate) async fn login(server_url: &ServerUrl) -> Result<(), AuthError> {
    let http_client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    let authorization = start_device_authorization(&http_client, server_url).await?;

    println!("Open this URL to authorize the CLI:");
    println!("{}", authorization.verification_uri);
    println!();
    println!("Enter code: {}", authorization.user_code);
    println!();

    if let Err(error) = webbrowser::open(authorization.verification_uri_complete.as_str()) {
        eprintln!("warning: could not open a browser: {error}");
    }

    println!("Waiting for authorization...");
    let token = poll_for_token(&http_client, server_url, &authorization).await?;
    save_credentials(server_url, &token)?;
    println!("Logged in to {server_url}");
    Ok(())
}

pub(crate) fn print_status(server_url: &ServerUrl) -> Result<(), AuthError> {
    let path = credentials_path()?;
    let Some(credentials) = load_credentials_from(&path)? else {
        println!("Not logged in.");
        return Ok(());
    };

    if credentials.server_url != *server_url.as_url() {
        println!("Logged in to {}.", credentials.server_url);
        println!("The selected server is {server_url}.");
        return Ok(());
    }

    let now = unix_timestamp()?;
    if credentials.expires_at <= now {
        println!("The login for {server_url} has expired.");
    } else {
        println!("Logged in to {server_url}.");
        println!(
            "The credential expires in {} seconds.",
            credentials.expires_at - now
        );
    }

    Ok(())
}

pub(crate) async fn logout(server_url: &ServerUrl) -> Result<(), AuthError> {
    let path = credentials_path()?;
    let Some(credentials) = load_credentials_from(&path)? else {
        println!("Not logged in.");
        return Ok(());
    };

    if credentials.server_url != *server_url.as_url() {
        return Err(AuthError::ServerMismatch {
            saved: credentials.server_url.to_string(),
            requested: server_url.to_string(),
        });
    }

    let http_client = Client::builder().timeout(Duration::from_secs(30)).build()?;
    revoke_access_token(&http_client, server_url, credentials.access_token.as_str()).await?;

    fs::remove_file(&path).map_err(|source| AuthError::CredentialIo { path, source })?;
    println!("Logged out.");
    Ok(())
}

pub(crate) fn access_token(server_url: &ServerUrl) -> Result<AccessToken, AuthError> {
    let path = credentials_path()?;
    let credentials = load_credentials_from(&path)?.ok_or(AuthError::NotLoggedIn)?;

    if credentials.server_url != *server_url.as_url() {
        return Err(AuthError::ServerMismatch {
            saved: credentials.server_url.to_string(),
            requested: server_url.to_string(),
        });
    }
    if credentials.expires_at <= unix_timestamp()? {
        return Err(AuthError::CredentialsExpired);
    }

    Ok(AccessToken(credentials.access_token))
}

async fn start_device_authorization(
    client: &Client,
    server_url: &ServerUrl,
) -> Result<DeviceAuthorizationResponse, AuthError> {
    let endpoint = server_url.device_authorization_url();
    let device_name = device_name();
    let response = client
        .post(endpoint)
        .form(&[
            ("client_id", CLIENT_ID),
            ("device_name", device_name.as_str()),
        ])
        .send()
        .await?;
    let status = response.status();

    if status.is_success() {
        return response.json().await.map_err(AuthError::from);
    }

    let error = response.json::<OAuthErrorResponse>().await.ok();
    match error {
        Some(error) => Err(AuthError::Protocol(error.error)),
        None => Err(AuthError::UnexpectedResponse {
            endpoint: "/oauth/device/authorization",
            status,
        }),
    }
}

async fn revoke_access_token(
    client: &Client,
    server_url: &ServerUrl,
    access_token: &str,
) -> Result<(), AuthError> {
    let endpoint = server_url.revocation_url();
    let response = client
        .post(endpoint)
        .form(&[
            ("token", access_token),
            ("token_type_hint", TOKEN_TYPE_HINT),
            ("client_id", CLIENT_ID),
        ])
        .send()
        .await?;
    let status = response.status();

    if status.is_success() {
        return Ok(());
    }

    let error = response.json::<OAuthErrorResponse>().await.ok();
    match error {
        Some(error) => Err(AuthError::Protocol(error.error)),
        None => Err(AuthError::UnexpectedResponse {
            endpoint: "/oauth/revoke",
            status,
        }),
    }
}

fn device_name() -> String {
    hostname::get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .and_then(|name| normalize_device_name(&name))
        .unwrap_or_else(|| format!("Opsd CLI on {}", std::env::consts::OS))
}

fn normalize_device_name(name: &str) -> Option<String> {
    let name = name.trim();
    if name.is_empty()
        || name.chars().count() > MAX_DEVICE_NAME_CHARACTERS
        || name.chars().any(char::is_control)
    {
        return None;
    }

    Some(name.to_string())
}

async fn poll_for_token(
    client: &Client,
    server_url: &ServerUrl,
    authorization: &DeviceAuthorizationResponse,
) -> Result<TokenResponse, AuthError> {
    let endpoint = server_url.token_url();
    let deadline = Instant::now() + Duration::from_secs(authorization.expires_in);
    let mut interval = Duration::from_secs(authorization.interval.max(1));

    loop {
        if Instant::now() + interval >= deadline {
            return Err(AuthError::DeviceAuthorizationExpired);
        }
        sleep(interval).await;

        let response = client
            .post(endpoint.clone())
            .form(&[
                ("grant_type", DEVICE_GRANT_TYPE),
                ("device_code", authorization.device_code.as_str()),
                ("client_id", CLIENT_ID),
            ])
            .send()
            .await?;
        let status = response.status();

        if status.is_success() {
            let token = response.json::<TokenResponse>().await?;
            if !token.token_type.eq_ignore_ascii_case("bearer") || token.access_token.is_empty() {
                return Err(AuthError::UnsupportedTokenType);
            }
            return Ok(token);
        }

        let error = response.json::<OAuthErrorResponse>().await.ok();
        match error.as_ref().map(|error| poll_action(&error.error)) {
            Some(Ok(PollAction::Pending)) => {}
            Some(Ok(PollAction::SlowDown)) => {
                interval = interval.saturating_add(SLOW_DOWN_INCREMENT)
            }
            Some(Ok(PollAction::Expired)) => {
                return Err(AuthError::DeviceAuthorizationExpired);
            }
            Some(Err(error)) => return Err(error),
            None => {
                return Err(AuthError::UnexpectedResponse {
                    endpoint: "/oauth/token",
                    status,
                });
            }
        }
    }
}

fn poll_action(error: &str) -> Result<PollAction, AuthError> {
    match error {
        "authorization_pending" => Ok(PollAction::Pending),
        "slow_down" => Ok(PollAction::SlowDown),
        "expired_token" => Ok(PollAction::Expired),
        error => Err(AuthError::Protocol(error.to_string())),
    }
}

fn save_credentials(server_url: &ServerUrl, token: &TokenResponse) -> Result<(), AuthError> {
    let now = unix_timestamp()?;
    let expires_at = now
        .checked_add(token.expires_in)
        .ok_or(AuthError::InvalidSystemTime)?;
    let credentials = StoredCredentials {
        server_url: server_url.as_url().clone(),
        access_token: token.access_token.clone(),
        expires_at,
    };
    save_credentials_to(&credentials_path()?, &credentials)
}

fn credentials_path() -> Result<PathBuf, AuthError> {
    let override_directory = env::var_os("OPSCTL_CONFIG_DIR").map(PathBuf::from);
    credentials_path_from(override_directory, dirs::config_dir())
}

fn credentials_path_from(
    override_directory: Option<PathBuf>,
    platform_config_directory: Option<PathBuf>,
) -> Result<PathBuf, AuthError> {
    if let Some(directory) = override_directory {
        return Ok(directory.join("credentials.json"));
    }

    let directory = platform_config_directory.ok_or(AuthError::ConfigDirectoryUnavailable)?;
    Ok(directory.join("opsctl").join("credentials.json"))
}

fn load_credentials_from(path: &Path) -> Result<Option<StoredCredentials>, AuthError> {
    #[cfg(unix)]
    match fs::metadata(path) {
        Ok(metadata) if metadata.permissions().mode() & 0o077 != 0 => {
            return Err(AuthError::InsecureCredentialPermissions {
                path: path.to_path_buf(),
            });
        }
        Ok(_) => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(AuthError::CredentialIo {
                path: path.to_path_buf(),
                source,
            });
        }
    }

    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(source) => {
            return Err(AuthError::CredentialIo {
                path: path.to_path_buf(),
                source,
            });
        }
    };

    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_error| AuthError::InvalidCredentials {
            path: path.to_path_buf(),
        })
}

fn save_credentials_to(path: &Path, credentials: &StoredCredentials) -> Result<(), AuthError> {
    let directory = path.parent().ok_or(AuthError::ConfigDirectoryUnavailable)?;
    fs::create_dir_all(directory).map_err(|source| AuthError::CredentialIo {
        path: directory.to_path_buf(),
        source,
    })?;

    #[cfg(unix)]
    fs::set_permissions(directory, fs::Permissions::from_mode(0o700)).map_err(|source| {
        AuthError::CredentialIo {
            path: directory.to_path_buf(),
            source,
        }
    })?;

    let temporary_suffix = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_error| AuthError::InvalidSystemTime)?
        .as_nanos();
    let temporary_path =
        path.with_extension(format!("{}.{}.tmp", std::process::id(), temporary_suffix));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        options.mode(0o600);

        let mut file = options
            .open(&temporary_path)
            .map_err(|source| AuthError::CredentialIo {
                path: temporary_path.clone(),
                source,
            })?;
        serde_json::to_writer(&mut file, credentials).map_err(|error| AuthError::CredentialIo {
            path: temporary_path.clone(),
            source: io::Error::other(error),
        })?;
        file.write_all(b"\n")
            .and_then(|()| file.sync_all())
            .map_err(|source| AuthError::CredentialIo {
                path: temporary_path.clone(),
                source,
            })?;
        fs::rename(&temporary_path, path).map_err(|source| AuthError::CredentialIo {
            path: path.to_path_buf(),
            source,
        })?;
        Ok(())
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temporary_path);
    }
    result
}

fn unix_timestamp() -> Result<u64, AuthError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|_error| AuthError::InvalidSystemTime)
}

#[cfg(test)]
mod tests {
    use super::{
        AccessToken, AuthError, PollAction, ServerUrl, StoredCredentials, credentials_path_from,
        load_credentials_from, normalize_device_name, poll_action, save_credentials_to,
    };
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::{fs, path::PathBuf};
    use tempfile::tempdir;
    use url::Url;

    #[test]
    fn production_server_url_derives_application_routes() {
        let server = ServerUrl::production();

        assert_eq!(server.as_url().as_str(), "https://api.opsd.sh/");
        assert_eq!(
            server.device_authorization_url().as_str(),
            "https://api.opsd.sh/oauth/device/authorization"
        );
        assert_eq!(
            server.public_api_base_url().as_str(),
            "https://api.opsd.sh/v1/"
        );
        assert_eq!(
            server.revocation_url().as_str(),
            "https://api.opsd.sh/oauth/revoke"
        );
    }

    #[test]
    fn server_urls_reject_embedded_credentials_and_non_http_schemes() {
        assert!(matches!(
            ServerUrl::from_override(Url::parse("https://user@example.com/").unwrap()),
            Err(AuthError::InvalidServerUrl(_))
        ));
        assert!(matches!(
            ServerUrl::from_override(Url::parse("ftp://api.opsd.sh/").unwrap()),
            Err(AuthError::InvalidServerUrl(_))
        ));
        assert!(matches!(
            ServerUrl::from_override(Url::parse("https://api.opsd.sh/v1/").unwrap()),
            Err(AuthError::InvalidServerUrl(_))
        ));
    }

    #[test]
    fn device_polling_errors_have_explicit_actions() {
        assert_eq!(
            poll_action("authorization_pending").unwrap(),
            PollAction::Pending
        );
        assert_eq!(poll_action("slow_down").unwrap(), PollAction::SlowDown);
        assert_eq!(poll_action("expired_token").unwrap(), PollAction::Expired);
        assert!(matches!(
            poll_action("invalid_client"),
            Err(AuthError::Protocol(error)) if error == "invalid_client"
        ));
    }

    #[test]
    fn access_token_debug_output_is_redacted() {
        let token = AccessToken("secret-token".to_string());

        assert_eq!(format!("{token:?}"), "AccessToken([REDACTED])");
    }

    #[test]
    fn device_names_are_trimmed_and_validated() {
        assert_eq!(
            normalize_device_name("  Kevin’s MacBook  ").as_deref(),
            Some("Kevin’s MacBook")
        );
        assert!(normalize_device_name("line\nbreak").is_none());
        assert!(normalize_device_name("   ").is_none());
        assert!(normalize_device_name(&"a".repeat(81)).is_none());
    }

    #[test]
    fn credentials_use_the_opsctl_platform_config_directory() {
        let path = credentials_path_from(None, Some(PathBuf::from("platform-config"))).unwrap();

        assert_eq!(
            path,
            PathBuf::from("platform-config")
                .join("opsctl")
                .join("credentials.json")
        );
    }

    #[test]
    fn credentials_config_override_replaces_the_platform_directory() {
        let path = credentials_path_from(
            Some(PathBuf::from("override-config")),
            Some(PathBuf::from("platform-config")),
        )
        .unwrap();

        assert_eq!(
            path,
            PathBuf::from("override-config").join("credentials.json")
        );
    }

    #[test]
    fn credentials_require_a_config_directory() {
        assert!(matches!(
            credentials_path_from(None, None),
            Err(AuthError::ConfigDirectoryUnavailable)
        ));
    }

    #[test]
    fn credentials_round_trip_without_relaxing_permissions() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("nested").join("credentials.json");
        let credentials = StoredCredentials {
            server_url: Url::parse("https://api.opsd.sh/").unwrap(),
            access_token: "secret-token".to_string(),
            expires_at: 123,
        };

        save_credentials_to(&path, &credentials).unwrap();
        let loaded = load_credentials_from(&path).unwrap().unwrap();

        assert_eq!(loaded.server_url, credentials.server_url);
        assert_eq!(loaded.access_token, credentials.access_token);
        assert_eq!(loaded.expires_at, credentials.expires_at);
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[cfg(unix)]
    #[test]
    fn credentials_with_broad_permissions_are_rejected() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("credentials.json");
        fs::write(&path, b"{}").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(matches!(
            load_credentials_from(&path),
            Err(AuthError::InsecureCredentialPermissions { .. })
        ));
    }
}
