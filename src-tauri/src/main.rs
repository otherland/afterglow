#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod credits;
mod github;

use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{TrayIcon, TrayIconBuilder},
    Manager,
};

struct Meter {
    tray: TrayIcon,
    summary: MenuItem<tauri::Wry>,
    details: Vec<MenuItem<tauri::Wry>>,
    refresh: MenuItem<tauri::Wry>,
    config_path: PathBuf,
    busy: AtomicBool,
}

impl Meter {
    fn update(&self, result: Result<credits::Credits, String>) -> tauri::Result<()> {
        match result {
            Ok(data) => {
                let quota = data.credits;
                let summary = format!(
                    "{}: {:.1}%",
                    credits::level(quota.percent_remaining),
                    quota.percent_remaining
                );
                self.summary.set_text(&summary)?;
                self.tray
                    .set_tooltip(Some(format!("Afterglow — {summary}")))?;
                #[cfg(target_os = "macos")]
                self.tray
                    .set_title(Some(format!("{:.1}%", quota.percent_remaining)))?;
                let checked = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_err(|e| tauri::Error::Io(std::io::Error::other(e)))?
                    .as_secs();
                let utc_seconds = checked % 86400;
                for (item, text) in self.details.iter().zip([
                    format!("Credits remaining: {}", quota.quota_remaining),
                    format!("Monthly allowance: {}", quota.entitlement),
                    format!("Credits used: {}", quota.credits_used),
                    format!("Resets: {}", data.resets),
                    format!("Plan: {}", data.plan),
                    format!(
                        "API reported: {}",
                        quota
                            .timestamp_utc
                            .as_deref()
                            .unwrap_or("timestamp unavailable")
                    ),
                    format!(
                        "Last checked: {:02}:{:02} UTC",
                        utc_seconds / 3600,
                        utc_seconds % 3600 / 60
                    ),
                ]) {
                    item.set_text(text)?;
                }
                eprintln!("Afterglow: tray refreshed successfully");
            }
            Err(error) => {
                eprintln!("Afterglow: {error}");
                self.summary
                    .set_text("Balance unavailable — see error below")?;
                self.tray
                    .set_tooltip(Some("Afterglow — balance unavailable"))?;
                #[cfg(target_os = "macos")]
                self.tray.set_title(Some("?"))?;
                for item in &self.details {
                    item.set_text("Unavailable — refresh to retry")?;
                }
                self.details[0].set_text(format!("Last error: {error}"))?;
            }
        }
        Ok(())
    }

    fn request(self: &Arc<Self>) {
        if self.busy.swap(true, Ordering::SeqCst) {
            return;
        }
        let meter = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            if let Err(e) = meter.refresh.set_enabled(false) {
                eprintln!("Afterglow: cannot disable Refresh: {e}");
            }
            let result = match std::fs::read(&meter.config_path)
                .map_err(|e| format!("Cannot read {}: {e}", meter.config_path.display()))
                .and_then(|bytes| {
                    serde_json::from_slice::<github::Config>(&bytes)
                        .map_err(|e| format!("Invalid config.json: {e}"))
                }) {
                Ok(config) => github::fetch(&config).await,
                Err(error) => Err(error),
            };
            if let Err(e) = meter.update(result) {
                eprintln!("Afterglow: cannot update tray: {e}");
            }
            if let Err(e) = meter.refresh.set_enabled(true) {
                eprintln!("Afterglow: cannot enable Refresh: {e}");
            }
            meter.busy.store(false, Ordering::SeqCst);
        });
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let config_dir = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config_dir)?;
            let config_path = config_dir.join("config.json");
            if !config_path.exists() {
                std::fs::write(
                    &config_path,
                    serde_json::to_vec_pretty(&github::Config::default())?,
                )?;
            }
            let summary = MenuItem::with_id(
                app,
                "summary",
                "Checking Copilot credits…",
                false,
                None::<&str>,
            )?;
            let details = (0..7)
                .map(|i| {
                    MenuItem::with_id(app, format!("detail-{i}"), "Loading…", false, None::<&str>)
                })
                .collect::<tauri::Result<Vec<_>>>()?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh now", true, None::<&str>)?;
            let config =
                MenuItem::with_id(app, "config", "Open settings file…", true, None::<&str>)?;
            let about = MenuItem::with_id(
                app,
                "about",
                "Afterglow 0.1.0 · Unofficial · 10-minute refresh",
                false,
                None::<&str>,
            )?;
            let quit = MenuItem::with_id(app, "quit", "Quit Afterglow", true, None::<&str>)?;
            let menu = Menu::new(app)?;
            menu.append(&summary)?;
            for item in &details {
                menu.append(item)?;
            }
            for item in [&refresh, &config, &about, &quit] {
                menu.append(item)?;
            }
            // A monochrome ring is legible as a template icon in both macOS appearances.
            let mut rgba = vec![0u8; 22 * 22 * 4];
            for y in 0..22 {
                for x in 0..22 {
                    let distance = ((x as f64 - 10.5).powi(2) + (y as f64 - 10.5).powi(2)).sqrt();
                    if (6.0..9.0).contains(&distance) {
                        let i = (y * 22 + x) * 4;
                        rgba[i..i + 4].copy_from_slice(&[110, 110, 110, 255]);
                    }
                }
            }
            let tray = TrayIconBuilder::with_id("afterglow")
                .icon(tauri::image::Image::new_owned(rgba, 22, 22))
                .icon_as_template(true)
                .tooltip("Afterglow — checking credits")
                .menu(&menu)
                .show_menu_on_left_click(true)
                .build(app)?;
            let meter = Arc::new(Meter {
                tray,
                summary,
                details,
                refresh,
                config_path,
                busy: AtomicBool::new(false),
            });
            let actions = Arc::clone(&meter);
            app.on_menu_event(move |app, event| match event.id.as_ref() {
                "quit" => app.exit(0),
                "refresh" => actions.request(),
                "config" => {
                    #[cfg(target_os = "macos")]
                    let result = std::process::Command::new("open")
                        .arg("-t")
                        .arg(&actions.config_path)
                        .spawn();
                    #[cfg(target_os = "windows")]
                    let result = std::process::Command::new("notepad.exe")
                        .arg(&actions.config_path)
                        .spawn();
                    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
                    let result = std::process::Command::new("xdg-open")
                        .arg(&actions.config_path)
                        .spawn();
                    if let Err(error) = result {
                        if let Err(e) =
                            actions.update(Err(format!("Cannot open settings: {error}")))
                        {
                            eprintln!("Afterglow: cannot display settings error: {e}");
                        }
                    }
                }
                _ => {}
            });
            meter.request();
            tauri::async_runtime::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(600)).await;
                    meter.request();
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("Afterglow could not start");
}
