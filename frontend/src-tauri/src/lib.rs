mod model;
mod process;
mod tools;
mod updater;

use model::{AppError, AppResult, DownloadKind, DownloadRequest, Settings};
use process::{execute, log_notice, resolve_tools, task_event, TaskRegistry};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
use tauri::{AppHandle, Manager, State};

pub struct AppState {
    settings: Mutex<Settings>,
    settings_file: PathBuf,
    tasks: TaskRegistry,
}

// App identifier rename (2.3.3) moved every app directory from the old
// com.ssk-shandm.ytdlp-gui name to com.ssk-shandm.grabmeta. Both live on the same
// volume, so the upgrade renames the legacy directories instead of copying them:
// that is instant and keeps settings.json plus the WebView2 profile (localStorage,
// cookies, cache) intact. Must run before the webview is created.
const LEGACY_CONFIG_DIR: &str = "com.ssk-shandm.ytdlp-gui";
const CURRENT_CONFIG_DIR: &str = "com.ssk-shandm.grabmeta";

/// Rename legacy app directories to the new identifier.
///
/// The rename keeps the WebView2 profile (localStorage, cookies, cache) and the
/// settings file without copying ~86 MiB. Windows refuses to rename a directory
/// while a process still holds a handle inside it; at this point the previous
/// version has exited, so the usual blockers are a lingering `msedgewebview2`
/// child or an antivirus scan of the freshly written files. Retry for a couple of
/// seconds to ride those out, then fall back safely.
///
/// If the rename still fails, nothing is destroyed: the legacy directory stays put,
/// `migrate_legacy_settings` copies `settings.json` across, and the webview starts a
/// fresh profile. The only loss in that fallback is the auto-update toggle (which
/// defaults to enabled) and the webview cache.
pub fn migrate_legacy_directories() {
    for variable in ["APPDATA", "LOCALAPPDATA"] {
        let Some(root) = std::env::var_os(variable) else {
            continue;
        };
        let root = PathBuf::from(root);
        let legacy = root.join(LEGACY_CONFIG_DIR);
        let current = root.join(CURRENT_CONFIG_DIR);
        if !legacy.is_dir() || current.exists() {
            continue;
        }
        // 10 attempts with a growing delay (~2.7 s total) covers transient locks
        // without noticeably delaying a normal first launch.
        for attempt in 0..10 {
            if std::fs::rename(&legacy, &current).is_ok() || !legacy.is_dir() {
                break;
            }
            if attempt < 9 {
                std::thread::sleep(std::time::Duration::from_millis(120 + attempt * 40));
            }
        }
    }
}

fn migrate_legacy_settings(config: &Path, settings_file: &Path) {
    if settings_file.exists() {
        return;
    }
    let Some(parent) = config.parent() else { return };
    let legacy = parent.join(LEGACY_CONFIG_DIR).join("settings.json");
    if !legacy.is_file() {
        return;
    }
    let Ok(bytes) = std::fs::read(&legacy) else { return };
    // Only carry over a file that still parses and validates.
    let Ok(settings) = serde_json::from_slice::<Settings>(&bytes) else {
        return;
    };
    if validate_settings(&settings).is_err() {
        return;
    }
    let _ = std::fs::write(settings_file, &bytes);
}

fn validate_settings(settings: &Settings) -> AppResult<()> {
    model::validate_retries(&settings.retry_times)?;
    model::validate_concurrent_fragments(settings.concurrent_fragments)?;
    if settings.proxy_enabled {
        model::validate_proxy_url(&settings.proxy_url)?;
    }
    if !PathBuf::from(&settings.download_path).is_absolute() {
        return Err(AppError::new("settings.pathNotAbsolute"));
    }
    if settings.download_path.contains('\0') {
        return Err(AppError::new("settings.pathInvalid"));
    }
    Ok(())
}
#[tauri::command]
fn get_settings(state: State<AppState>) -> AppResult<Settings> {
    state
        .settings
        .lock()
        .map(|s| s.clone())
        .map_err(|_| AppError::new("settings.lockUnavailable"))
}
#[tauri::command]
fn save_settings(settings: Settings, state: State<AppState>) -> AppResult<Settings> {
    validate_settings(&settings)?;
    std::fs::create_dir_all(&settings.download_path).map_err(|e| {
        AppError::with("settings.directoryCreateFailed", e.to_string())
    })?;
    let mut current = state
        .settings
        .lock()
        .map_err(|_| AppError::new("settings.lockUnavailable"))?;
    let json = serde_json::to_vec_pretty(&settings)
        .map_err(|e| AppError::with("settings.saveFailed", e.to_string()))?;
    std::fs::write(&state.settings_file, json)
        .map_err(|e| AppError::with("settings.saveFailed", e.to_string()))?;
    *current = settings.clone();
    Ok(settings)
}
#[tauri::command]
async fn analyze_url(
    url: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<serde_json::Value> {
    let url = model::validate_url(&url)?;
    let tools = resolve_tools(&app)?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| AppError::new("settings.lockUnavailable"))?
        .clone();
    let mut args = model::proxy_args(&settings)?;
    args.extend(
        [
            "--ignore-config",
            "--encoding",
            "utf-8",
            "--no-color",
            "--no-playlist",
            "--skip-download",
            "--dump-single-json",
            "--socket-timeout",
            "30",
            "--retries",
            "3",
            "--",
            &url,
        ]
        .into_iter()
        .map(str::to_string),
    );
    let (id, control) = state.tasks.reserve()?;
    task_event(&app, id, "analyze", "running", AppError::new("task.analyzing"));
    let result = execute(&app, &tools.ytdlp, &args, id, control, true)
        .await
        .and_then(|bytes| {
            serde_json::from_slice(&bytes)
                .map(model::normalize_metadata)
                .map_err(|e| AppError::with("metadata.parseFailed", e.to_string()))
        });
    state.tasks.finish(id);
    match &result {
        Ok(_) => task_event(&app, id, "analyze", "success", AppError::new("task.analyzed")),
        Err(e) => {
            log_notice(&app, id, e.clone());
            let status = if e.code == "task.cancelled" {
                "cancelled"
            } else {
                "error"
            };
            task_event(&app, id, "analyze", status, e.clone());
        }
    }
    result
}

fn launch(
    app: AppHandle,
    executable: PathBuf,
    args: Vec<String>,
    kind: &'static str,
) -> AppResult<u64> {
    let state = app.state::<AppState>();
    let (id, control) = state.tasks.reserve()?;
    task_event(&app, id, kind, "running", AppError::new("task.started"));
    log_notice(&app, id, AppError::with("log.started", kind));
    let worker_app = app.clone();
    tauri::async_runtime::spawn(async move {
        let result = execute(&worker_app, &executable, &args, id, control, false).await;
        worker_app.state::<AppState>().tasks.finish(id);
        match result {
            Ok(_) => {
                log_notice(&worker_app, id, AppError::with("log.completed", kind));
                task_event(&worker_app, id, kind, "success", AppError::new("task.completed"));
            }
            Err(e) => {
                let status = if e.code == "task.cancelled" {
                    "cancelled"
                } else {
                    "error"
                };
                log_notice(&worker_app, id, e.clone());
                task_event(&worker_app, id, kind, status, e);
            }
        }
    });
    Ok(id)
}
#[tauri::command]
fn start_download(
    request: DownloadRequest,
    app: AppHandle,
    state: State<AppState>,
) -> AppResult<u64> {
    let tools = resolve_tools(&app)?;
    let settings = state
        .settings
        .lock()
        .map_err(|_| AppError::new("settings.lockUnavailable"))?
        .clone();
    std::fs::create_dir_all(&settings.download_path).map_err(|e| {
        AppError::with("settings.directoryCreateFailed", e.to_string())
    })?;
    let args = model::download_args(&request, &settings, &tools.ffmpeg_dir.to_string_lossy())?;
    let kind = match request.kind {
        DownloadKind::Quick => "quick",
        DownloadKind::Format => "format",
        DownloadKind::Combined => "combined",
        DownloadKind::Subtitle => "subtitle",
        DownloadKind::Thumbnail => "thumbnail",
        DownloadKind::Description => "description",
    };
    launch(app, tools.ytdlp, args, kind)
}
#[tauri::command]
fn list_supported_sites(app: AppHandle) -> AppResult<u64> {
    let tools = resolve_tools(&app)?;
    launch(
        app,
        tools.ytdlp,
        vec![
            "--ignore-config".into(),
            "--encoding".into(),
            "utf-8".into(),
            "--no-color".into(),
            "--list-extractors".into(),
        ],
        "sites",
    )
}
#[tauri::command]
fn cancel_task(task_id: u64, state: State<AppState>) -> AppResult<()> {
    state.tasks.cancel(task_id)
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(updater::UpdateState::default())
        .manage(tools::ToolInstallState::default())
        .setup(|app| {
            let config = app.path().app_config_dir()?;
            std::fs::create_dir_all(&config)?;
            let settings_file = config.join("settings.json");
            // If the directory rename could not happen (locked files, different
            // volume), still carry over the settings file written by a pre-2.3.3
            // install so the user keeps their download directory and preferences.
            migrate_legacy_settings(&config, &settings_file);
            let default = Settings {
                download_path: app
                    .path()
                    .download_dir()
                    .or_else(|_| app.path().home_dir().map(|p| p.join("Downloads")))?
                    .to_string_lossy()
                    .into_owned(),
                retry_times: "10".into(),
                concurrent_fragments: model::default_concurrent_fragments(),
                proxy_enabled: false,
                proxy_url: model::default_proxy_url(),
            };
            let settings = std::fs::read(&settings_file)
                .ok()
                .and_then(|bytes| serde_json::from_slice::<Settings>(&bytes).ok())
                .filter(|s| validate_settings(s).is_ok())
                .unwrap_or(default);
            app.manage(AppState {
                settings: Mutex::new(settings),
                settings_file,
                tasks: TaskRegistry::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            analyze_url,
            start_download,
            list_supported_sites,
            cancel_task,
            updater::fetch_latest_release,
            updater::download_and_install_update,
            updater::get_tool_info,
            tools::get_tool_status,
            tools::download_tools,
            tools::cancel_tool_download,
            tools::open_tool_licenses,
            tools::open_gui_licenses
        ])
        .build(tauri::generate_context!())
        .expect("failed to start GrabMeta");
    app.run(|app, event| {
        if matches!(
            event,
            tauri::RunEvent::ExitRequested { .. } | tauri::RunEvent::Exit
        ) {
            app.state::<tools::ToolInstallState>().cancel();
            app.state::<AppState>().tasks.shutdown();
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "grabmeta-migrate-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn valid_settings() -> Settings {
        Settings {
            download_path: r"C:\测试 下载".into(),
            retry_times: "3".into(),
            concurrent_fragments: 8,
            proxy_enabled: false,
            proxy_url: model::default_proxy_url(),
        }
    }

    #[test]
    fn legacy_settings_are_copied_when_directory_rename_did_not_happen() {
        let root = temp_root("copy");
        let legacy_dir = root.join(LEGACY_CONFIG_DIR);
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(
            legacy_dir.join("settings.json"),
            serde_json::to_vec(&valid_settings()).unwrap(),
        )
        .unwrap();
        let current_dir = root.join(CURRENT_CONFIG_DIR);
        std::fs::create_dir_all(&current_dir).unwrap();
        let settings_file = current_dir.join("settings.json");

        migrate_legacy_settings(&current_dir, &settings_file);

        let copied: Settings =
            serde_json::from_slice(&std::fs::read(&settings_file).unwrap()).unwrap();
        assert_eq!(copied.download_path, r"C:\测试 下载");
        assert_eq!(copied.retry_times, "3");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn existing_settings_are_never_overwritten_by_the_legacy_copy() {
        let root = temp_root("keep");
        let legacy_dir = root.join(LEGACY_CONFIG_DIR);
        std::fs::create_dir_all(&legacy_dir).unwrap();
        std::fs::write(
            legacy_dir.join("settings.json"),
            serde_json::to_vec(&valid_settings()).unwrap(),
        )
        .unwrap();
        let current_dir = root.join(CURRENT_CONFIG_DIR);
        std::fs::create_dir_all(&current_dir).unwrap();
        let settings_file = current_dir.join("settings.json");
        let mut current = valid_settings();
        current.download_path = r"D:\新目录".into();
        std::fs::write(&settings_file, serde_json::to_vec(&current).unwrap()).unwrap();

        migrate_legacy_settings(&current_dir, &settings_file);

        let kept: Settings =
            serde_json::from_slice(&std::fs::read(&settings_file).unwrap()).unwrap();
        assert_eq!(kept.download_path, r"D:\新目录");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn invalid_legacy_settings_are_not_migrated() {
        let root = temp_root("invalid");
        let legacy_dir = root.join(LEGACY_CONFIG_DIR);
        std::fs::create_dir_all(&legacy_dir).unwrap();
        // Relative download path fails validation.
        std::fs::write(
            legacy_dir.join("settings.json"),
            br#"{"downloadPath":"relative/path","retryTimes":"3"}"#,
        )
        .unwrap();
        let current_dir = root.join(CURRENT_CONFIG_DIR);
        std::fs::create_dir_all(&current_dir).unwrap();
        let settings_file = current_dir.join("settings.json");

        migrate_legacy_settings(&current_dir, &settings_file);

        assert!(!settings_file.exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn missing_legacy_directory_is_harmless() {
        let root = temp_root("missing");
        let current_dir = root.join(CURRENT_CONFIG_DIR);
        std::fs::create_dir_all(&current_dir).unwrap();
        let settings_file = current_dir.join("settings.json");

        migrate_legacy_settings(&current_dir, &settings_file);

        assert!(!settings_file.exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn directory_rename_moves_settings_and_webview_profile_together() {
        // Mirrors what migrate_legacy_directories does for APPDATA / LOCALAPPDATA.
        let root = temp_root("rename");
        let legacy = root.join(LEGACY_CONFIG_DIR);
        std::fs::create_dir_all(legacy.join("EBWebView")).unwrap();
        std::fs::write(
            legacy.join("settings.json"),
            serde_json::to_vec(&valid_settings()).unwrap(),
        )
        .unwrap();
        std::fs::write(legacy.join("EBWebView").join("Cookies"), b"keep").unwrap();
        let current = root.join(CURRENT_CONFIG_DIR);

        assert!(std::fs::rename(&legacy, &current).is_ok());

        assert!(!legacy.exists());
        assert!(current.join("settings.json").is_file());
        assert_eq!(
            std::fs::read(current.join("EBWebView").join("Cookies")).unwrap(),
            b"keep"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rename_is_skipped_when_the_new_directory_already_exists() {
        let root = temp_root("skip");
        let legacy = root.join(LEGACY_CONFIG_DIR);
        let current = root.join(CURRENT_CONFIG_DIR);
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(legacy.join("settings.json"), b"{}").unwrap();

        // Guard used by migrate_legacy_directories.
        if legacy.is_dir() && !current.exists() {
            std::fs::rename(&legacy, &current).unwrap();
        }

        assert!(legacy.exists(), "legacy directory must stay untouched");
        assert!(!current.join("settings.json").exists());
        std::fs::remove_dir_all(&root).ok();
    }
}
