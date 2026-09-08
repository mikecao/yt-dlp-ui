#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[cfg(debug_assertions)]
use tauri::Manager;

mod hosts;
mod logins;
mod premium;

fn main() {
    let builder = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            logins::save_login_password,
            logins::get_login_password,
            logins::delete_login_password,
            logins::list_saved_logins,
            premium::start_premium_download,
            premium::cancel_premium_download,
            premium::submit_login_verification,
        ])
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_sql::Builder::default()
                .add_migrations(
                    "sqlite:getdown.db",
                    vec![tauri_plugin_sql::Migration {
                        version: 1,
                        description: "create settings table",
                        sql: "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    }, tauri_plugin_sql::Migration {
                        version: 2,
                        description: "create saved logins metadata table",
                        sql: "CREATE TABLE IF NOT EXISTS saved_logins (domain TEXT PRIMARY KEY, username TEXT NOT NULL, include_subdomains INTEGER NOT NULL DEFAULT 0)",
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    }, tauri_plugin_sql::Migration {
                        version: 3,
                        description: "encrypt saved credentials in SQLite",
                        sql: "ALTER TABLE saved_logins ADD COLUMN credentials BLOB; ALTER TABLE saved_logins ADD COLUMN legacy_cleanup INTEGER NOT NULL DEFAULT 1;",
                        kind: tauri_plugin_sql::MigrationKind::Up,
                    }],
                )
                .build(),
        );

    // `pnpm dev` starts Vite first and sets TAURI_DEV_URL to the bound port.
    // The compiled config still has a default URL, so navigate at runtime.
    #[cfg(debug_assertions)]
    let builder = builder.setup(|app| {
        if let (Ok(url), Some(window)) = (
            std::env::var("TAURI_DEV_URL"),
            app.get_webview_window("main"),
        ) {
            let parsed: tauri::Url = url.parse().expect("invalid TAURI_DEV_URL");
            window.navigate(parsed)?;
        }
        Ok(())
    });

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
