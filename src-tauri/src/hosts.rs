//! Premium host protocols. Passwords and signed URLs stay in the native process.
use chrono::{DateTime, Utc};
use reqwest::{Client, Response};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, LazyLock,
    },
    time::{Duration, Instant},
};
use tauri::Url;
use tokio::sync::Mutex;

const PREMIUM_REQUIRED: &str =
    "An active premium account is required. Free downloads are not supported.";
const QUOTA: &str = "The premium account has no download traffic remaining.";
const CHALLENGE: &str = "The host requires a CAPTCHA that could not be completed here. Check your account on its website, then retry.";

#[derive(Clone)]
struct Session {
    client: Client,
    token: String,
    created: Instant,
}
static SESSIONS: LazyLock<Mutex<HashMap<String, Session>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

static ACCOUNT_LOCKS: LazyLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
static SESSION_REVISION: AtomicU64 = AtomicU64::new(0);

pub async fn clear_sessions() {
    SESSION_REVISION.fetch_add(1, Ordering::SeqCst);
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
    if message.contains("captcha") {
        CHALLENGE.into()
    } else if message.contains("verif") || message.contains("auth code") {
        "The host requires additional login verification. Complete any account checks on its website, then retry. This challenge is not supported in the app.".into()
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

fn k2_error(value: &Value, method: &str) -> String {
    let code = value["errorCode"]
        .as_u64()
        .or_else(|| value["errorCode"].as_str()?.parse().ok());
    // Map documented codes before interpreting prose. Never echo server messages,
    // which may contain credentials, tokens, or signed URLs.
    let message = match code {
        Some(33) if method == "login" => return CHALLENGE.into(),
        Some(10) => "The host denied authorization for this request.",
        Some(71) => "Too many login attempts. Wait before trying again.",
        Some(72) => "The host has banned this account. Contact the host's support.",
        Some(73) => "The host does not allow login from this network.",
        Some(74) => "The host could not log in to this account. Check that your saved email and password work on its website.",
        Some(75) => "The host rejected the session's IP address. Check whether your network or VPN changed.",
        Some(76) => "The host flagged this account as compromised. Recover access on its website.",
        Some(77) => "The host has blocked this network. Contact the host's support.",
        _ => match value["errorCode"].as_str() {
            Some("captcha_need_wait") => "The host temporarily blocked CAPTCHA attempts. Wait before trying again.",
            Some("captcha_need_wait_daily") => "The host blocked CAPTCHA attempts from this IP address. Contact the host's support.",
            _ => {
                let error = protocol_error(value);
                if error == CHALLENGE && method == "login" { return error; }
                return match code {
                    Some(code) => format!("{method} failed (host error {code}): {error}"),
                    None => format!("{method} failed: {error}"),
                };
            }
        },
    };
    match code {
        Some(code) => format!("{method} failed (host error {code}): {message}"),
        None => format!("{method} failed: {message}"),
    }
}

async fn json_response(
    response: Response,
    rapidgator: bool,
    method: &str,
) -> Result<Value, String> {
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
        return Err(if rapidgator {
            protocol_error(&value)
        } else {
            k2_error(&value, method)
        });
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
    json_response(response, true, method).await
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
    json_response(response, false, method).await
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

async fn login(
    domain: &str,
    credentials: crate::logins::Credentials,
    download_id: &str,
    events: &tauri::ipc::Channel<crate::premium::DownloadEvent>,
) -> Result<Session, String> {
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
            let mut body =
                json!({"username": credentials.username, "password": credentials.password});
            let mut result = k2(&client, domain, "login", body.clone()).await;
            for _ in 0..3 {
                if !matches!(&result, Err(error) if error == CHALLENGE) {
                    break;
                }
                let challenge = k2(&client, domain, "requestReCaptcha", json!({})).await?;
                let challenge_id = challenge["challenge"]
                    .as_str()
                    .ok_or("The host did not return a login challenge.")?;
                let mut url = Url::parse(
                    challenge["captcha_url"]
                        .as_str()
                        .ok_or("The host did not return a verification page.")?,
                )
                .map_err(|_| "The host returned an invalid verification page.")?;
                let expected_host = if domain == "k2s.cc" {
                    "keep2share.cc"
                } else {
                    "tezfiles.com"
                };
                if url.host_str() != Some(expected_host)
                    || !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                    || url.port().is_some()
                {
                    return Err(
                        "The host returned an unexpected verification page. Login was stopped."
                            .into(),
                    );
                }
                url.set_scheme("https")
                    .map_err(|_| "Invalid verification page.")?;
                let code =
                    crate::premium::request_verification(download_id, url.to_string(), events)
                        .await?;
                body["re_captcha_challenge"] = json!(challenge_id);
                body["re_captcha_response"] = json!(code);
                result = k2(&client, domain, "login", body.clone()).await;
            }
            let info = result.map_err(|error| if error == CHALLENGE {
                "The host rejected the verification code. Start the download again for a fresh challenge.".into()
            } else { error })?;
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

pub async fn resolve(
    app: &tauri::AppHandle,
    input: &str,
    credential_domain: &str,
    download_id: &str,
    events: &tauri::ipc::Channel<crate::premium::DownloadEvent>,
) -> Result<PremiumFile, String> {
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

    let account_lock = ACCOUNT_LOCKS
        .lock()
        .await
        .entry(credential_domain.into())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone();
    let _account_guard = account_lock.lock().await;
    let revision = SESSION_REVISION.load(Ordering::SeqCst);
    let credentials =
        crate::logins::get_login_password(app.clone(), credential_domain.to_string()).await?;
    // Human verification holds only this account's lock, not the whole session cache.
    let cached = SESSIONS
        .lock()
        .await
        .get(credential_domain)
        .cloned()
        .filter(|s| s.created.elapsed() <= Duration::from_secs(1800));
    let session = match cached {
        Some(session) => session,
        None => login(domain, credentials, download_id, events).await?,
    };
    if let Err(error) = check_account(&session, domain).await {
        SESSIONS.lock().await.remove(credential_domain);
        return Err(error);
    }
    {
        let mut sessions = SESSIONS.lock().await;
        if SESSION_REVISION.load(Ordering::SeqCst) != revision {
            return Err(
                "Saved logins changed during verification. Start the download again.".into(),
            );
        }
        sessions.insert(credential_domain.into(), session.clone());
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
            // File metadata is public. Supplying the optional token can cause
            // this endpoint to reject an otherwise accessible file. Premium
            // access is checked above and the download URL stays authenticated.
            let info = k2(&session.client, domain, "getFileStatus", json!({"id": id})).await?;
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
