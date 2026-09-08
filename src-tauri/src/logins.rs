use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng, Payload},
    Aes256Gcm, Nonce,
};
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::sync::{LazyLock, Mutex};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex as AsyncMutex;
use zeroize::Zeroizing;

static VAULT_LOCK: AsyncMutex<()> = AsyncMutex::const_new(());
static KEYRING_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
const LEGACY_SERVICE: &str = "get-down.saved-logins";
const KEY_SERVICE: &str = "get-down.credential-encryption";
const KEY_ACCOUNT: &str = "sqlite-master-key-v1";
const KEY_ERROR: &str = "Could not unlock saved logins. Unlock the system credential store or restore the original encryption key. A database backup alone cannot unlock these passwords.";
const DB_ERROR: &str = "Could not access the saved-logins database.";

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub(crate) username: String,
    pub(crate) password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedLogin {
    domain: String,
    username: String,
    storage_warning: Option<String>,
}

async fn pool(app: &AppHandle) -> Result<SqlitePool, String> {
    let state = app.state::<tauri_plugin_sql::DbInstances>();
    let instances = state.0.read().await;
    match instances.get("sqlite:getdown.db") {
        Some(tauri_plugin_sql::DbPool::Sqlite(pool)) => Ok(pool.clone()),
        _ => Err(DB_ERROR.into()),
    }
}

fn validate_domain(domain: &str) -> Result<(), String> {
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
    Ok(())
}

async fn encryption_key(pool: &SqlitePool) -> Result<Zeroizing<Vec<u8>>, String> {
    // Serialize first-key creation across app processes sharing this database.
    let mut transaction = pool
        .begin_with("BEGIN IMMEDIATE")
        .await
        .map_err(|_| DB_ERROR)?;
    let count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM saved_logins WHERE credentials IS NOT NULL")
            .fetch_one(&mut *transaction)
            .await
            .map_err(|_| DB_ERROR)?;
    let key =
        tauri::async_runtime::spawn_blocking(move || -> Result<Zeroizing<Vec<u8>>, String> {
            let _guard = KEYRING_LOCK.lock().map_err(|_| KEY_ERROR)?;
            let entry = keyring::Entry::new(KEY_SERVICE, KEY_ACCOUNT).map_err(|_| KEY_ERROR)?;
            let key = match entry.get_secret() {
                Ok(key) => Zeroizing::new(key),
                Err(keyring::Error::NoEntry) if count == 0 => {
                    let key = Zeroizing::new(Aes256Gcm::generate_key(OsRng).to_vec());
                    entry.set_secret(&key).map_err(|_| KEY_ERROR)?;
                    // Verify persistence before encrypting anything with this key.
                    let persisted = Zeroizing::new(entry.get_secret().map_err(|_| KEY_ERROR)?);
                    if *persisted != *key {
                        return Err(KEY_ERROR.into());
                    }
                    key
                }
                _ => return Err(KEY_ERROR.into()),
            };
            if key.len() != 32 {
                return Err(KEY_ERROR.into());
            }
            Ok(key)
        })
        .await
        .map_err(|_| KEY_ERROR.to_string())??;
    transaction.commit().await.map_err(|_| DB_ERROR)?;
    Ok(key)
}

async fn verify_key(pool: &SqlitePool, key: &[u8]) -> Result<(), String> {
    if let Some(row) = sqlx::query(
        "SELECT domain, credentials FROM saved_logins WHERE credentials IS NOT NULL LIMIT 1",
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| DB_ERROR)?
    {
        decrypt(
            key,
            row.get("domain"),
            &row.get::<Vec<u8>, _>("credentials"),
        )?;
    }
    Ok(())
}

fn encrypt(key: &[u8], domain: &str, credentials: &Credentials) -> Result<Vec<u8>, String> {
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| KEY_ERROR)?;
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let plaintext =
        Zeroizing::new(serde_json::to_vec(credentials).map_err(|_| "Could not encode login.")?);
    // Bind ciphertext to its domain. Format: version byte, 12-byte nonce, ciphertext + tag.
    let encrypted = cipher
        .encrypt(
            &nonce,
            Payload {
                msg: &plaintext,
                aad: domain.as_bytes(),
            },
        )
        .map_err(|_| "Could not encrypt login.")?;
    let mut result = vec![1];
    result.extend_from_slice(&nonce);
    result.extend_from_slice(&encrypted);
    Ok(result)
}

fn decrypt(key: &[u8], domain: &str, encrypted: &[u8]) -> Result<Credentials, String> {
    let error = "Could not decrypt the saved login. The encryption key was changed or the database record is damaged.";
    if encrypted.len() < 29 || encrypted[0] != 1 {
        return Err(error.into());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| error)?;
    let plaintext = Zeroizing::new(
        cipher
            .decrypt(
                Nonce::from_slice(&encrypted[1..13]),
                Payload {
                    msg: &encrypted[13..],
                    aad: domain.as_bytes(),
                },
            )
            .map_err(|_| error)?,
    );
    serde_json::from_slice(&plaintext).map_err(|_| error.into())
}

async fn legacy_read(domain: String) -> Result<Credentials, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = KEYRING_LOCK.lock().map_err(|_| "Credential store is busy.")?;
        let entry = keyring::Entry::new(LEGACY_SERVICE, &domain).map_err(|_| "Could not access the old login.")?;
        let value = Zeroizing::new(entry.get_password().map_err(|_| "The old saved password is unavailable. Unlock the system credential store or enter the password again to migrate this login.")?);
        serde_json::from_str(&value).map_err(|_| "The old login could not be read. Enter the password again.".into())
    }).await.map_err(|_| "Could not read the old login.".to_string())?
}

async fn cleanup_legacy(pool: &SqlitePool, domain: &str) -> Result<(), String> {
    let owned_domain = domain.to_string();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = KEYRING_LOCK.lock().map_err(|_| "Credential store is busy.")?;
        let entry = keyring::Entry::new(LEGACY_SERVICE, &owned_domain).map_err(|_| "Could not access the old login.")?;
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(_) => Err("The old system-store entry could not be removed. The database record has been retained. Unlock the credential store and reopen Settings to retry cleanup.".to_string()),
        }
    }).await.map_err(|_| "Could not remove the old login.".to_string())??;
    sqlx::query("UPDATE saved_logins SET legacy_cleanup = 0 WHERE domain = ?")
        .bind(domain)
        .execute(pool)
        .await
        .map_err(|_| DB_ERROR)?;
    Ok(())
}

async fn migrate(pool: &SqlitePool, domain: &str) -> Result<(), String> {
    let row = sqlx::query("SELECT credentials, legacy_cleanup FROM saved_logins WHERE domain = ?")
        .bind(domain)
        .fetch_one(pool)
        .await
        .map_err(|_| DB_ERROR)?;
    let encrypted: Option<Vec<u8>> = row.get("credentials");
    if encrypted.is_some() && row.get::<i64, _>("legacy_cleanup") == 0 {
        return Ok(());
    }
    let key = encryption_key(pool).await?;
    if let Some(encrypted) = encrypted {
        // On retry, verify the committed copy before removing the legacy credential.
        decrypt(&key, domain, &encrypted)?;
    } else {
        verify_key(pool, &key).await?;
        let credentials = legacy_read(domain.to_string()).await?;
        let encrypted = encrypt(&key, domain, &credentials)?;
        decrypt(&key, domain, &encrypted)?;
        sqlx::query("UPDATE saved_logins SET username = ?, credentials = ? WHERE domain = ?")
            .bind(&credentials.username)
            .bind(&encrypted)
            .bind(domain)
            .execute(pool)
            .await
            .map_err(|_| DB_ERROR)?;
    }
    cleanup_legacy(pool, domain).await
}

#[tauri::command]
pub async fn list_saved_logins(app: AppHandle) -> Result<Vec<SavedLogin>, String> {
    let _guard = VAULT_LOCK.lock().await;
    let pool = pool(&app).await?;
    let domains: Vec<String> =
        sqlx::query_scalar("SELECT domain FROM saved_logins ORDER BY domain")
            .fetch_all(&pool)
            .await
            .map_err(|_| DB_ERROR)?;
    let mut result = Vec::new();
    for domain in domains {
        let warning = migrate(&pool, &domain).await.err();
        let row = sqlx::query("SELECT username FROM saved_logins WHERE domain = ?")
            .bind(&domain)
            .fetch_one(&pool)
            .await
            .map_err(|_| DB_ERROR)?;
        result.push(SavedLogin {
            domain,
            username: row.get("username"),
            storage_warning: warning,
        });
    }
    Ok(result)
}

#[tauri::command]
pub async fn get_login_password(app: AppHandle, domain: String) -> Result<Credentials, String> {
    validate_domain(&domain)?;
    let _guard = VAULT_LOCK.lock().await;
    let pool = pool(&app).await?;
    // A cleanup failure does not prevent using a successfully migrated encrypted record.
    let migration = migrate(&pool, &domain).await;
    let encrypted: Option<Vec<u8>> =
        sqlx::query_scalar("SELECT credentials FROM saved_logins WHERE domain = ?")
            .bind(&domain)
            .fetch_one(&pool)
            .await
            .map_err(|_| "Saved login not found.")?;
    let encrypted = match encrypted {
        Some(value) => value,
        None => {
            return Err(migration
                .err()
                .unwrap_or_else(|| "Enter the password again.".into()))
        }
    };
    let key = encryption_key(&pool).await?;
    decrypt(&key, &domain, &encrypted)
}

#[tauri::command]
pub async fn save_login_password(
    app: AppHandle,
    domain: String,
    username: String,
    password: String,
    editing: bool,
) -> Result<SavedLogin, String> {
    validate_domain(&domain)?;
    let username = username.trim().to_string();
    if username.is_empty() {
        return Err("Enter a username.".into());
    }
    let guard = VAULT_LOCK.lock().await;
    let pool = pool(&app).await?;
    let existing =
        sqlx::query("SELECT credentials, legacy_cleanup FROM saved_logins WHERE domain = ?")
            .bind(&domain)
            .fetch_optional(&pool)
            .await
            .map_err(|_| DB_ERROR)?;
    if existing.is_some() && !editing {
        return Err("A login already exists for this domain. Use Edit to update it.".into());
    }
    if existing.is_none() && editing {
        return Err("This login no longer exists. Add it again.".into());
    }
    let key = encryption_key(&pool).await?;
    // Detect a replaced (but well-formed) master key before writing more records.
    verify_key(&pool, &key).await?;
    let password = if password.is_empty() {
        match existing
            .as_ref()
            .and_then(|r| r.get::<Option<Vec<u8>>, _>("credentials"))
        {
            Some(value) => decrypt(&key, &domain, &value)?.password,
            None if existing.is_some() => legacy_read(domain.clone()).await?.password,
            None => return Err("Enter a password.".into()),
        }
    } else {
        password
    };
    let credentials = Credentials {
        username: username.clone(),
        password,
    };
    let encrypted = encrypt(&key, &domain, &credentials)?;
    let pending = existing
        .as_ref()
        .is_some_and(|r| r.get::<i64, _>("legacy_cleanup") != 0);
    if editing {
        let updated =
            sqlx::query("UPDATE saved_logins SET username = ?, credentials = ? WHERE domain = ?")
                .bind(&username)
                .bind(encrypted)
                .bind(&domain)
                .execute(&pool)
                .await
                .map_err(|_| DB_ERROR)?;
        if updated.rows_affected() == 0 {
            return Err("This login no longer exists. Add it again.".into());
        }
    } else {
        sqlx::query("INSERT INTO saved_logins (domain, username, credentials, legacy_cleanup) VALUES (?, ?, ?, 0)")
            .bind(&domain).bind(&username).bind(encrypted)
            .execute(&pool).await.map_err(|_| "Could not add the login. Reload Settings to check whether this domain already exists.")?;
    }
    let storage_warning = if pending {
        cleanup_legacy(&pool, &domain).await.err()
    } else {
        None
    };
    drop(guard);
    crate::hosts::clear_sessions().await;
    Ok(SavedLogin {
        domain,
        username,
        storage_warning,
    })
}

#[tauri::command]
pub async fn delete_login_password(app: AppHandle, domain: String) -> Result<(), String> {
    validate_domain(&domain)?;
    let guard = VAULT_LOCK.lock().await;
    let pool = pool(&app).await?;
    let pending: Option<i64> =
        sqlx::query_scalar("SELECT legacy_cleanup FROM saved_logins WHERE domain = ?")
            .bind(&domain)
            .fetch_optional(&pool)
            .await
            .map_err(|_| DB_ERROR)?;
    if pending.is_some_and(|n| n != 0) {
        cleanup_legacy(&pool, &domain).await?;
    }
    sqlx::query("DELETE FROM saved_logins WHERE domain = ?")
        .bind(domain)
        .execute(&pool)
        .await
        .map_err(|_| DB_ERROR)?;
    drop(guard);
    crate::hosts::clear_sessions().await;
    Ok(())
}
