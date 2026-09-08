use serde::Serialize;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, Mutex},
    time::{Duration, Instant},
};
use tauri::ipc::Channel;
use tokio::{io::AsyncWriteExt, sync::oneshot};

static DOWNLOADS: LazyLock<Mutex<HashMap<String, oneshot::Sender<()>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum DownloadEvent {
    Log {
        message: String,
    },
    Progress {
        name: String,
        downloaded: u64,
        total: Option<u64>,
        speed: f64,
    },
    Complete {
        name: String,
    },
    Error {
        message: String,
    },
}

#[tauri::command]
pub fn start_premium_download(
    url: String,
    credential_domain: String,
    save_path: String,
    rate_limit: String,
    events: Channel<DownloadEvent>,
) -> Result<String, String> {
    let limit = parse_rate(&rate_limit)?;
    let id = uuid::Uuid::new_v4().to_string();
    let (cancel, cancelled) = oneshot::channel();
    DOWNLOADS
        .lock()
        .map_err(|_| "Download manager is unavailable.")?
        .insert(id.clone(), cancel);
    let task_id = id.clone();
    tauri::async_runtime::spawn(async move {
        let result = tokio::select! {
            result = download(&task_id, &url, &credential_domain, &save_path, limit, &events) => Some(result),
            _ = cancelled => None,
        };
        if let Some(Err(message)) = result {
            let _ = events.send(DownloadEvent::Error { message });
        }
        if let Ok(mut downloads) = DOWNLOADS.lock() {
            downloads.remove(&task_id);
        }
    });
    Ok(id)
}

#[tauri::command]
pub fn cancel_premium_download(id: String) -> Result<(), String> {
    if let Some(cancel) = DOWNLOADS
        .lock()
        .map_err(|_| "Download manager is unavailable.")?
        .remove(&id)
    {
        let _ = cancel.send(());
    }
    Ok(())
}

fn parse_rate(input: &str) -> Result<Option<f64>, String> {
    let input = input.trim();
    if input.is_empty() {
        return Ok(None);
    }
    let (value, scale) = match input.as_bytes().last().map(|c| c.to_ascii_uppercase()) {
        Some(b'K') => (&input[..input.len() - 1], 1024.0),
        Some(b'M') => (&input[..input.len() - 1], 1024.0 * 1024.0),
        Some(b'G') => (&input[..input.len() - 1], 1024.0 * 1024.0 * 1024.0),
        _ => (input, 1.0),
    };
    let rate = value
        .parse::<f64>()
        .ok()
        .map(|n| n * scale)
        .filter(|n| n.is_finite() && *n >= 1.0 && *n <= 1e12)
        .ok_or("Enter a valid rate limit, such as 500K or 2M.")?;
    Ok(Some(rate))
}

fn filename(name: &str) -> String {
    // Never treat a host-provided filename as a filesystem path.
    let name: String = name
        .chars()
        .map(|c| {
            if c.is_control() || "<>:\"/\\|?*".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    let mut name = name.trim().trim_end_matches(['.', ' ']).to_string();
    // Leave room for our partial-file suffix on filesystems with 255-byte names.
    while name.len() > 180 {
        name.pop();
    }
    name = name.trim_end_matches(['.', ' ']).to_string();
    if name.is_empty() {
        name = "download".into();
    }
    let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit())
    {
        name.insert(0, '_');
    }
    name
}

async fn download(
    id: &str,
    url: &str,
    credential_domain: &str,
    save_path: &str,
    rate_limit: Option<f64>,
    events: &Channel<DownloadEvent>,
) -> Result<(), String> {
    let _ = events.send(DownloadEvent::Log {
        message: "Checking premium account and file access…".into(),
    });
    let file = crate::hosts::resolve(url, credential_domain).await?;
    let name = filename(&file.name);
    let folder = PathBuf::from(if save_path.is_empty() { "." } else { save_path });
    tokio::fs::create_dir_all(&folder)
        .await
        .map_err(|_| "Could not create the download folder.")?;
    let target = folder.join(&name);
    if tokio::fs::try_exists(&target)
        .await
        .map_err(|_| "Could not inspect the destination file.")?
    {
        if !tokio::fs::metadata(&target)
            .await
            .map_err(|_| "Could not inspect the destination file.")?
            .is_file()
        {
            return Err(
                "A folder already exists with the download filename. Choose another save folder."
                    .into(),
            );
        }
        let _ = events.send(DownloadEvent::Log {
            message: format!("Skipped existing file: {name}"),
        });
        let _ = events.send(DownloadEvent::Complete { name });
        return Ok(());
    }
    let mut response = crate::hosts::get_response(&file.client, &file.url).await?;
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_lowercase();
    let attachment = response
        .headers()
        .get("content-disposition")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.to_lowercase().starts_with("attachment"));
    if !attachment
        && (content_type.contains("text/html")
            || content_type.contains("application/json")
            || content_type.contains("application/xhtml"))
    {
        return Err("The host returned an error page instead of a file. Check your premium access and quota, then retry.".into());
    }
    let length = response.content_length();
    if let (Some(expected), Some(actual)) = (file.size, length) {
        if expected != actual {
            return Err("The download size does not match the host's file information. The download was stopped.".into());
        }
    }
    let total = file.size.or(length);
    let partial = folder.join(format!("{name}.{id}.part"));
    let mut output = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&partial)
        .await
        .map_err(|_| {
            "Could not create a partial download file. Check folder permissions and free space."
        })?;
    let _ = events.send(DownloadEvent::Log {
        message: format!("Destination: {name}"),
    });
    let started = Instant::now();
    let mut last_update = Instant::now();
    let mut downloaded = 0u64;
    while let Some(chunk) = response.chunk().await.map_err(|_| "The file transfer was interrupted. The partial file was kept; retry to start a new download.")? {
        downloaded += chunk.len() as u64;
        if total.is_some_and(|size| downloaded > size) { return Err("The host sent more data than expected. The partial file was kept.".into()); }
        output.write_all(&chunk).await.map_err(|_| "Could not write the download. Check available disk space. The partial file was kept.")?;
        if let Some(rate) = rate_limit {
            let expected = Duration::from_secs_f64(downloaded as f64 / rate);
            if let Some(wait) = expected.checked_sub(started.elapsed()) { tokio::time::sleep(wait).await; }
        }
        if last_update.elapsed() >= Duration::from_millis(250) {
            events.send(DownloadEvent::Progress { name: name.clone(), downloaded, total,
                speed: downloaded as f64 / started.elapsed().as_secs_f64().max(0.001) })
                .map_err(|_| "The download window was closed. The partial file was kept.")?;
            last_update = Instant::now();
        }
    }
    if total.is_some_and(|size| downloaded != size) {
        return Err(
            "The download ended before the complete file arrived. The partial file was kept."
                .into(),
        );
    }
    output
        .flush()
        .await
        .map_err(|_| "Could not flush the downloaded file to disk.")?;
    output
        .sync_all()
        .await
        .map_err(|_| "Could not save the downloaded file to disk.")?;
    drop(output);
    let _ = events.send(DownloadEvent::Log {
        message: "Saving completed download…".into(),
    });
    // Finish publishing even if cancellation arrives during finalization. At this
    // point all bytes are present and the task must not leave an incomplete final file.
    tauri::async_runtime::spawn_blocking(move || publish(partial, target))
        .await
        .map_err(|_| "Could not finalize the download. The .part file was kept.")??;
    let _ = events.send(DownloadEvent::Progress {
        name: name.clone(),
        downloaded,
        total: Some(downloaded),
        speed: 0.0,
    });
    let _ = events.send(DownloadEvent::Complete { name });
    Ok(())
}

fn publish(partial: PathBuf, target: PathBuf) -> Result<(), String> {
    // Hard links atomically publish without overwriting concurrent downloads.
    if std::fs::hard_link(&partial, &target).is_err() {
        // FAT/exFAT and some network filesystems do not support hard links.
        // create_new still guarantees that an existing destination is untouched.
        let mut destination = std::fs::OpenOptions::new().write(true).create_new(true).open(&target)
            .map_err(|_| "Could not finalize the download. The destination may already exist. The complete .part file was kept.")?;
        let copied = (|| -> std::io::Result<()> {
            let mut source = std::fs::File::open(&partial)?;
            std::io::copy(&mut source, &mut destination)?;
            destination.sync_all()
        })();
        drop(destination);
        if copied.is_err() {
            // This file was created exclusively by this operation.
            let _ = std::fs::remove_file(&target);
            return Err("Could not finalize the download. Check disk space. The complete .part file was kept.".into());
        }
    }
    let _ = std::fs::remove_file(&partial);
    Ok(())
}
