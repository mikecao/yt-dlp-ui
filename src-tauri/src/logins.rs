use serde::{Deserialize, Serialize};
use std::sync::Mutex;

// Serialize access because native credential stores do not reliably support
// concurrent operations on the same entry.
static CREDENTIAL_LOCK: Mutex<()> = Mutex::new(());
const SERVICE: &str = "get-down.saved-logins";

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: String,
}

fn entry(domain: &str) -> Result<keyring::Entry, String> {
    let url =
        tauri::Url::parse(&format!("https://{domain}")).map_err(|_| "Invalid login domain.")?;
    if url.host_str() != Some(domain)
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("Invalid login domain.".into());
    }
    keyring::Entry::new(SERVICE, domain)
        .map_err(|_| "Could not access the system credential store.".into())
}

#[tauri::command]
pub async fn save_login_password(
    domain: String,
    username: String,
    password: String,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || save_password(domain, username, password))
        .await
        .map_err(|_| "Could not access the system credential store.".to_string())??;
    crate::hosts::clear_sessions().await;
    Ok(())
}

fn save_password(domain: String, username: String, password: String) -> Result<(), String> {
    if username.trim().is_empty() || password.is_empty() {
        return Err("A username and password are required.".into());
    }
    let _guard = CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential store is busy.")?;
    let value = serde_json::to_string(&Credentials { username, password })
        .map_err(|_| "Could not encode the login.")?;
    entry(&domain)?.set_password(&value).map_err(|_| {
        "Could not save the login. Unlock your system credential store and try again.".into()
    })
}

#[tauri::command]
pub async fn get_login_password(domain: String) -> Result<Credentials, String> {
    tauri::async_runtime::spawn_blocking(move || read_password(domain))
        .await
        .map_err(|_| "Could not access the system credential store.".to_string())?
}

fn read_password(domain: String) -> Result<Credentials, String> {
    let _guard = CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential store is busy.")?;
    let value = entry(&domain)?.get_password()
        .map_err(|_| "Could not read the saved login. Unlock your system credential store or save the login again.".to_string())?;
    serde_json::from_str(&value)
        .map_err(|_| "The saved login could not be read. Save it again.".into())
}

#[tauri::command]
pub async fn delete_login_password(domain: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || delete_password(domain))
        .await
        .map_err(|_| "Could not access the system credential store.".to_string())??;
    crate::hosts::clear_sessions().await;
    Ok(())
}

fn delete_password(domain: String) -> Result<(), String> {
    let _guard = CREDENTIAL_LOCK
        .lock()
        .map_err(|_| "Credential store is busy.")?;
    match entry(&domain)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(_) => Err(
            "Could not remove the login. Unlock your system credential store and try again.".into(),
        ),
    }
}
