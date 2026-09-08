//! Premium host protocols. Passwords and signed URLs stay in the native process.
use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::LazyLock,
    time::{Duration, Instant},
};
use tauri::Url;
use tokio::sync::Mutex;

const PREMIUM_REQUIRED: &str =
    "An active premium account is required. Free downloads are not supported.";
const QUOTA: &str = "The premium account has no download traffic remaining.";
const CHALLENGE: &str = "The host requires login verification or a CAPTCHA. Complete any account checks on its website, then retry. Interactive login challenges are not supported yet.";

struct Session {
    client: Client,
    token: String,
    created: Instant,
}
static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub async fn clear_sessions() {
    SESSIONS.lock().await.clear();
}

pub fn host_domain(host: &str) -> Option<&'static str> {
    match host
        .trim_end_matches('.')
        .strip_prefix("www.")
        .unwrap_or(host.trim_end_matches('.'))
    {
        "rapidgator.net" => Some("rapidgator.net"),
        "k2s.cc" | "keep2share.cc" => Some("k2s.cc"),
        "tezfiles.com" => Some("tezfiles.com"),
        _ => None,
    }
}

fn client() -> Result<Client, String> {
    Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(60))
        .user_agent("GetDown/0.12 premium-downloader")
        .build()
        .map_err(|_| "Could not initialize the download client.".into())
}

fn network_error(_: reqwest::Error) -> String {
    // reqwest errors may contain URLs with passwords, tokens or signatures.
    "Could not reach the host. Check your connection and try again.".into()
}

fn protocol_error(value: &Value) -> String {
    let message = ["message", "details", "response_details", "error"]
        .iter()
        .filter_map(|key| value[*key].as_str())
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    if message.contains("captcha") || message.contains("verif") || message.contains("auth code") {
        CHALLENGE.into()
    } else if message.contains("traffic") || message.contains("quota") || message.contains("limit")
    {
        "The host reports a traffic or request limit. Check your premium account and retry later."
            .into()
    } else if message.contains("premium") || message.contains("subscription") {
        "Premium access to this file was denied. Check your subscription for this host or publisher.".into()
    } else if message.contains("not found") || message.contains("deleted") {
        "The requested file is unavailable or was removed.".into()
    } else if message.contains("password")
        || message.contains("auth")
        || message.contains("token")
        || message.contains("session")
        || message.contains("login")
    {
        "The host rejected the login or session. Check your saved credentials and retry.".into()
    } else {
        "The host rejected the request. Check your premium account and file access on its website."
            .into()
    }
}

async fn json_response(response: Response, rapidgator: bool) -> Result<Value, String> {
    let status = response.status();
    let value: Value = response.json().await.map_err(|_| {
        if status.as_u16() == 429 {
            "The host is rate limiting requests. Retry later.".into()
        } else {
            "The host returned an unexpected response or a browser challenge.".to_string()
        }
    })?;
    let code = value["status"]
        .as_u64()
        .or(value["response_status"].as_u64());
    let success = if rapidgator {
        code == Some(200) || value["success"] == true
    } else {
        value["status"] == "success" && value["code"] == 200
    };
    if !status.is_success() || !success {
        return Err(protocol_error(&value));
    }
    Ok(if rapidgator {
        value.get("response").cloned().unwrap_or(value)
    } else {
        value
    })
}

async fn rg(client: &Client, method: &str, params: &[(&str, &str)]) -> Result<Value, String> {
    let response = client
        .post(format!("https://rapidgator.net/api/v2/{method}"))
        .form(params)
        .send()
        .await
        .map_err(network_error)?;
    json_response(response, true).await
}

async fn k2(client: &Client, domain: &str, method: &str, body: Value) -> Result<Value, String> {
    let api_host = if domain == "k2s.cc" {
        "keep2share.cc"
    } else {
        "tezfiles.com"
    };
    let response = client
        .post(format!("https://{api_host}/api/v2/{method}"))
        .json(&body)
        .send()
        .await
        .map_err(network_error)?;
    json_response(response, false).await
}

fn expires_at(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value.as_str().and_then(|s| {
            s.parse::<i64>()
                .ok()
                .or_else(|| DateTime::parse_from_rfc3339(s).ok().map(|d| d.timestamp()))
        })
    })
}

async fn check_account(session: &Session, domain: &str) -> Result<(), String> {
    match domain {
        "rapidgator.net" => {
            let info = rg(&session.client, "user/info", &[("token", &session.token)]).await?;
            if info["user"]["is_premium"] != true {
                return Err(PREMIUM_REQUIRED.into());
            }
            if info["user"]["traffic"]["left"]
                .as_i64()
                .is_some_and(|n| n <= 0)
            {
                return Err(QUOTA.into());
            }
        }
        "k2s.cc" | "tezfiles.com" => {
            let info = k2(
                &session.client,
                domain,
                "accountInfo",
                json!({"auth_token": session.token}),
            )
            .await?;
            if expires_at(&info["account_expires"]).unwrap_or(0) <= Utc::now().timestamp() {
                return Err(PREMIUM_REQUIRED.into());
            }
            if info["available_traffic"].as_i64().is_some_and(|n| n <= 0) {
                return Err(QUOTA.into());
            }
        }
        _ => return Err("Unsupported premium host.".into()),
    }
    Ok(())
}

async fn login(domain: &str, credential_domain: &str) -> Result<Session, String> {
    let credentials = crate::logins::get_login_password(credential_domain.to_string()).await?;
    let client = client()?;
    let token = match domain {
        "rapidgator.net" => {
            let info = rg(
                &client,
                "user/login",
                &[
                    ("login", &credentials.username),
                    ("password", &credentials.password),
                ],
            )
            .await?;
            info["token"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("Rapidgator did not return a session token.")?
                .to_string()
        }
        "k2s.cc" | "tezfiles.com" => {
            let info = k2(
                &client,
                domain,
                "login",
                json!({"username": credentials.username, "password": credentials.password}),
            )
            .await?;
            info["auth_token"]
                .as_str()
                .filter(|s| !s.is_empty())
                .ok_or("The host did not return a session token.")?
                .to_string()
        }
        _ => return Err("Unsupported premium host.".into()),
    };
    Ok(Session {
        client,
        token,
        created: Instant::now(),
    })
}

pub struct PremiumFile {
    pub client: Client,
    pub url: String,
    pub name: String,
    pub size: Option<u64>,
}

pub async fn resolve(input: &str, credential_domain: &str) -> Result<PremiumFile, String> {
    let mut url = Url::parse(input).map_err(|_| "Invalid download URL.")?;
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err(
            "Enter a normal HTTP or HTTPS file link without credentials or a custom port.".into(),
        );
    }
    let domain = host_domain(url.host_str().unwrap_or("")).ok_or("Unsupported premium host.")?;
    if host_domain(credential_domain) != Some(domain) {
        return Err("Save a premium login for this host in Settings → Saved Logins.".into());
    }
    url.set_scheme("https")
        .map_err(|_| "Invalid download URL.")?;
    let parts: Vec<_> = url.path_segments().ok_or("Invalid file link.")?.collect();
    let id = match domain {
        "rapidgator.net" if parts.first() == Some(&"file") => parts.get(1),
        "k2s.cc" | "tezfiles.com" if parts.first() == Some(&"file") => parts.get(1),
        _ => None,
    }
    .filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric()))
    .ok_or("Paste a file download link for this host, not a folder or account page.")?
    .to_string();

    // Serialize login and link resolution, then release the lock before transferring bytes.
    let mut sessions = SESSIONS.lock().await;
    if sessions
        .get(credential_domain)
        .is_some_and(|s| s.created.elapsed() > Duration::from_secs(1800))
    {
        sessions.remove(credential_domain);
    }
    if !sessions.contains_key(credential_domain) {
        sessions.insert(
            credential_domain.into(),
            login(domain, credential_domain).await?,
        );
    }
    let session = &sessions[credential_domain];
    if let Err(error) = check_account(session, domain).await {
        sessions.remove(credential_domain);
        return Err(error);
    }
    let (direct, name, size) = match domain {
        "rapidgator.net" => {
            let info = rg(
                &session.client,
                "file/info",
                &[("token", &session.token), ("file_id", &id)],
            )
            .await?;
            let link = rg(
                &session.client,
                "file/download",
                &[("token", &session.token), ("file_id", &id)],
            )
            .await?;
            (
                link["download_url"]
                    .as_str()
                    .ok_or("Rapidgator did not return a premium download URL.")?
                    .to_string(),
                info["file"]["name"].as_str().unwrap_or(&id).to_string(),
                info["file"]["size"].as_u64(),
            )
        }
        "k2s.cc" | "tezfiles.com" => {
            let info = k2(
                &session.client,
                domain,
                "getFileStatus",
                json!({"id": id, "auth_token": session.token}),
            )
            .await?;
            if info["is_available"] != true || info["is_folder"] == true {
                return Err("The requested file is unavailable.".into());
            }
            let link = k2(
                &session.client,
                domain,
                "getUrl",
                json!({"file_id": id, "auth_token": session.token}),
            )
            .await?;
            // Never follow a free_download_key, countdown, or CAPTCHA response.
            (link["url"].as_str().ok_or("The host did not return a premium download URL. Check your subscription and quota.")?.to_string(),
             info["name"].as_str().unwrap_or(&id).to_string(), info["size"].as_u64())
        }
        _ => return Err("Unsupported premium host.".into()),
    };
    let client = session.client.clone();
    drop(sessions);
    Ok(PremiumFile {
        client,
        url: direct,
        name,
        size,
    })
}

pub async fn get_response(client: &Client, input: &str) -> Result<Response, String> {
    let mut url = Url::parse(input).map_err(|_| "The host returned an invalid download URL.")?;
    for _ in 0..10 {
        if url.scheme() != "https" || !url.username().is_empty() || url.password().is_some() {
            return Err(
                "The host returned an insecure download URL. The download was stopped.".into(),
            );
        }
        let response = client
            .get(url.clone())
            .send()
            .await
            .map_err(network_error)?;
        if response.status().is_redirection() {
            let location = response
                .headers()
                .get("location")
                .and_then(|v| v.to_str().ok())
                .ok_or("The host returned an invalid redirect.")?;
            url = url
                .join(location)
                .map_err(|_| "The host returned an invalid redirect.")?;
            continue;
        }
        if !response.status().is_success() {
            return Err(match response.status().as_u16() {
                401 => "The host rejected the login or download session.",
                402 | 403 => "The host denied access. Check your premium subscription, quota, and access to this file.",
                404 => "The requested file is unavailable or was removed.",
                429 => "The host is rate limiting requests. Retry later.",
                _ => "The host returned a server error. Retry later.",
            }.into());
        }
        return Ok(response);
    }
    Err("The host returned too many redirects.".into())
}
