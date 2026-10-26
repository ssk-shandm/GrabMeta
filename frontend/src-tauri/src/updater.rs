use crate::{
    model::{AppError, AppResult},
    process::tool_directory,
    AppState,
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs,
    io::{BufWriter, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Manager, State};

#[derive(Default)]
pub struct UpdateState {
    busy: AtomicBool,
}

struct Busy<'a>(&'a AtomicBool);

impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

impl UpdateState {
    fn begin(&self) -> AppResult<Busy<'_>> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| AppError::new("update.busy"))?;
        Ok(Busy(&self.busy))
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DownloadProgress {
    downloaded: u64,
    total: Option<u64>,
    percent: Option<u8>,
    file_name: String,
}

fn validate_update_identity(identifier: &str) -> AppResult<()> {
    if identifier != "com.ssk-shandm.grabmeta" {
        return Err(AppError::new("update.isolatedBuild"));
    }
    Ok(())
}

fn validate_update_url(url: &str) -> AppResult<()> {
    let parsed = reqwest::Url::parse(url).map_err(|_| AppError::new("update.invalidUrl"))?;
    if parsed.scheme() != "https"
        || parsed.host_str() != Some("github.com")
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.port().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(AppError::new("update.urlNotTrusted"));
    }
    let path = parsed.path();
    const PROJECT_RELEASE_PREFIX: &str = "/ssk-shandm/GrabMeta/releases/download/";
    let path = path
        .get(..PROJECT_RELEASE_PREFIX.len())
        .filter(|prefix| prefix.eq_ignore_ascii_case(PROJECT_RELEASE_PREFIX))
        .map(|_| &path[PROJECT_RELEASE_PREFIX.len()..])
        .ok_or_else(|| AppError::new("update.urlNotProjectRelease"))?;
    let parts: Vec<_> = path.split('/').collect();
    if parts.len() != 2 || parts.iter().any(|part| part.is_empty()) {
        return Err(AppError::new("update.invalidAssetPath"));
    }
    Ok(())
}

fn validate_update_sha256(value: Option<&str>) -> AppResult<String> {
    let value = value.ok_or_else(|| AppError::new("update.sha256Missing"))?;
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::new("update.sha256Invalid"));
    }
    Ok(value.to_ascii_lowercase())
}

fn verify_update_sha256(hash: Sha256, expected: &str) -> AppResult<()> {
    let actual = format!("{:x}", hash.finalize());
    if actual != expected {
        return Err(AppError::new("update.sha256Mismatch"));
    }
    Ok(())
}

// Created before the writer so error unwinding closes the file before cleanup.
struct PartialDownload(PathBuf);

impl Drop for PartialDownload {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn safe_update_file_name(file_name: &str) -> AppResult<String> {
    let name = file_name.trim();
    if name.contains(['/', '\\']) {
        return Err(AppError::new("update.fileNameSeparator"));
    }
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name.to_ascii_lowercase().ends_with("-setup.exe")
    {
        return Err(AppError::new("update.fileNameInvalid"));
    }
    if !name
        .chars()
        .all(|character| !character.is_control() && !r#"<>:"/\\|?*"#.contains(character))
    {
        return Err(AppError::new("update.fileNameUnsafe"));
    }
    Ok(name.to_string())
}

// Load the OS trust store (including user-trusted proxy/enterprise roots), and
// honor system proxies. Keep TLS verification enabled for executable downloads.
fn update_http_client(settings: Option<&crate::model::Settings>) -> AppResult<reqwest::Client> {
    let builder = reqwest::Client::builder();
    let builder = match settings {
        Some(settings) => crate::model::configure_http_proxy(builder, settings)?,
        None => builder,
    };
    builder
        .user_agent("grabmeta-updater")
        .https_only(true)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            let url = attempt.url();
            let allowed = matches!(
                url.host_str(),
                Some(
                    "api.github.com"
                        | "github.com"
                        | "objects.githubusercontent.com"
                        | "github-releases.githubusercontent.com"
                        | "release-assets.githubusercontent.com"
                )
            );
            if attempt.previous().len() >= 10 {
                attempt.error("too many update redirects")
            } else if url.scheme() != "https"
                || !allowed
                || !url.username().is_empty()
                || url.password().is_some()
                || url.port().is_some()
            {
                attempt.error("update redirect is not a trusted HTTPS GitHub endpoint")
            } else {
                attempt.follow()
            }
        }))
        .connect_timeout(Duration::from_secs(30))
        // Limit stalled reads, not the total duration of a large installer download.
        .read_timeout(Duration::from_secs(60))
        .build()
        .map_err(|error| AppError::with("update.clientInit", error_chain(&error)))
}

// Fixed endpoint only: the WebView cannot apply a per-request proxy.
#[tauri::command]
pub async fn fetch_latest_release(app: AppHandle) -> AppResult<serde_json::Value> {
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|_| AppError::new("update.settingsLocked"))?
        .clone();
    let response = update_http_client(Some(&settings))?
        .get("https://api.github.com/repos/ssk-shandm/GrabMeta/releases/latest")
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(20))
        .send()
        .await
        .map_err(|error| update_network_error(&error))?;
    match response.status().as_u16() {
        403 => return Err(AppError::new("update.rateLimited")),
        404 => return Err(AppError::new("update.noRelease")),
        _ if !response.status().is_success() => {
            return Err(AppError::with(
                "update.httpStatus",
                response.status().as_u16().to_string(),
            ))
        }
        _ => {}
    }
    response
        .json()
        .await
        .map_err(|error| AppError::with("update.responseInvalid", error_chain(&error)))
}

fn error_chain(error: &dyn Error) -> String {
    let mut details = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        details.push_str(" → ");
        details.push_str(&cause.to_string());
        source = cause.source();
    }
    details
}

fn update_network_error(error: &reqwest::Error) -> AppError {
    let details = error_chain(error);
    let lower = details.to_ascii_lowercase();
    if error.is_timeout() {
        AppError::with("update.network.timeout", details)
    } else if lower.contains("certificate") || lower.contains("unknownissuer") {
        AppError::with("update.network.tls", details)
    } else if error.is_connect() {
        AppError::with("update.network.connect", details)
    } else {
        AppError::with("update.network.generic", details)
    }
}

#[tauri::command]
pub async fn download_and_install_update(
    app: AppHandle,
    state: State<'_, UpdateState>,
    url: String,
    file_name: String,
    sha256: Option<String>,
) -> AppResult<()> {
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (app, state, url, file_name, sha256);
        return Err(AppError::new("update.windowsOnly"));
    }

    #[cfg(target_os = "windows")]
    {
        validate_update_identity(&app.config().identifier)?;
        let _busy = state.begin()?;
        validate_update_url(&url)?;
        let safe_name = safe_update_file_name(&file_name)?;
        let executable = std::env::current_exe()
            .map_err(|error| AppError::with("update.exePath", error.to_string()))?;
        let install_directory = executable
            .parent()
            .ok_or_else(|| AppError::new("update.installDirectory"))?;
        if !install_directory.join("uninstall.exe").is_file() {
            return Err(AppError::new("update.portableBuild"));
        }

        let expected_sha256 = validate_update_sha256(sha256.as_deref())?;
        app.state::<AppState>().tasks.begin_update()?;
        let result =
            download_update_package(&app, &url, &safe_name, install_directory, &expected_sha256)
                .await;
        if result.is_err() {
            app.state::<AppState>().tasks.end_update();
        }
        result
    }
}

#[cfg(target_os = "windows")]
async fn download_update_package(
    app: &AppHandle,
    url: &str,
    safe_name: &str,
    install_directory: &Path,
    expected_sha256: &str,
) -> AppResult<()> {
    let save_dir = app
        .path()
        .download_dir()
        .or_else(|_| app.path().temp_dir())
        .map_err(|error| AppError::with("update.saveDirUnavailable", error.to_string()))?;
    fs::create_dir_all(&save_dir)
        .map_err(|error| AppError::with("update.saveDirCreate", error.to_string()))?;
    let target = save_dir.join(safe_name);
    let partial = save_dir.join(format!("{safe_name}.part"));
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|_| AppError::new("update.settingsLocked"))?
        .clone();
    let response = update_http_client(Some(&settings))?
        .get(url)
        .header("Accept", "application/octet-stream")
        .send()
        .await
        .map_err(|error| update_network_error(&error))?;
    if !response.status().is_success() {
        return Err(AppError::with(
            "update.downloadHttpStatus",
            response.status().as_u16().to_string(),
        ));
    }

    let total = response.content_length();
    let mut downloaded = 0_u64;
    let mut hash = Sha256::new();
    let _partial = PartialDownload(partial.clone());
    let mut output = BufWriter::with_capacity(
        1024 * 1024,
        fs::File::create(&partial)
            .map_err(|error| AppError::with("update.fileCreate", error.to_string()))?,
    );
    let emit_progress = |downloaded: u64, percent: Option<u8>| {
        let _ = app.emit(
            "update-download-progress",
            DownloadProgress {
                downloaded,
                total,
                percent,
                file_name: safe_name.to_string(),
            },
        );
    };
    emit_progress(0, total.map(|_| 0));
    let mut last_percent = Some(0_u8);
    let mut last_reported_bytes = 0_u64;
    let mut response = response;
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| update_network_error(&error))?
    {
        hash.update(&chunk);
        output
            .write_all(&chunk)
            .map_err(|error| AppError::with("update.fileWrite", error.to_string()))?;
        downloaded += chunk.len() as u64;
        match total {
            Some(size) => {
                let percent = downloaded
                    .saturating_mul(100)
                    .checked_div(size)
                    .unwrap_or(100)
                    .min(100) as u8;
                if last_percent != Some(percent) {
                    last_percent = Some(percent);
                    emit_progress(downloaded, Some(percent));
                }
            }
            None => {
                if downloaded.saturating_sub(last_reported_bytes) >= 1024 * 1024 {
                    last_reported_bytes = downloaded;
                    emit_progress(downloaded, None);
                }
            }
        }
    }
    output
        .flush()
        .map_err(|error| AppError::with("update.fileWrite", error.to_string()))?;
    drop(output);
    if downloaded == 0 || total.is_some_and(|size| size != downloaded) {
        let _ = fs::remove_file(&partial);
        return Err(AppError::new("update.incomplete"));
    }
    verify_update_sha256(hash, expected_sha256)?;
    if total.is_none() {
        emit_progress(downloaded, None);
    }
    if target.exists() {
        let _ = fs::remove_file(&target);
    }
    fs::rename(&partial, &target)
        .map_err(|error| AppError::with("update.saveFailed", error.to_string()))?;
    if let Err(code) = launch_installer(&target, install_directory) {
        let path = target.display().to_string();
        return Err(if code == 5 {
            AppError::with("update.installerCancelled", path)
        } else {
            AppError::with("update.installerLaunch", path)
        });
    }
    let _ = app.emit(
        "update-install-starting",
        target.to_string_lossy().into_owned(),
    );
    app.exit(0);
    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_installer(path: &Path, install_directory: &Path) -> Result<(), isize> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    fn wide(value: &std::ffi::OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }

    let operation = wide(std::ffi::OsStr::new("open"));
    let file = wide(path.as_os_str());
    let mut arguments = std::ffi::OsString::from("/UPDATE /P /R /D=");
    arguments.push(install_directory);
    let parameters = wide(&arguments);
    let directory = path.parent().map(|dir| wide(dir.as_os_str()));
    let result = unsafe {
        ShellExecuteW(
            std::ptr::null_mut(),
            operation.as_ptr(),
            file.as_ptr(),
            parameters.as_ptr(),
            directory
                .as_ref()
                .map_or(std::ptr::null(), |dir| dir.as_ptr()),
            SW_SHOWNORMAL,
        )
    };
    let code = result as isize;
    if code > 32 {
        Ok(())
    } else {
        Err(code)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolInfo {
    bin_path: String,
    ffmpeg_version: Option<String>,
}

#[tauri::command]
pub async fn get_tool_info(app: AppHandle) -> AppResult<ToolInfo> {
    let dir = tool_directory(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let mut cmd = std::process::Command::new(dir.join("ffmpeg.exe"));
        cmd.arg("-version");
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000);
        }
        let output = cmd
            .output()
            .map_err(|error| AppError::with("update.ffmpegVersionRead", error.to_string()))?;
        if !output.status.success() {
            return Err(AppError::new("update.ffmpegVersionFailed"));
        }
        Ok(ToolInfo {
            bin_path: dir.to_string_lossy().into_owned(),
            ffmpeg_version: String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .map(str::to_string),
        })
    })
    .await
    .map_err(|error| AppError::with("update.ffmpegVersionRead", error.to_string()))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires network access to a GitHub Release asset"]
    async fn github_release_download_network_smoke() {
        let url = std::env::var("YT_DLP_GUI_UPDATE_TEST_URL")
            .expect("set YT_DLP_GUI_UPDATE_TEST_URL to a GitHub Release installer URL");
        validate_update_url(&url).unwrap();
        let result = update_http_client(None)
            .unwrap()
            .get(url)
            .header("Accept", "application/octet-stream")
            .send()
            .await;
        let response =
            result.unwrap_or_else(|error| panic!("{}", update_network_error(&error)));
        assert!(response.status().is_success(), "{}", response.status());
        let expected_length = response.content_length();
        let mut response = response;
        let mut downloaded = 0_u64;
        let mut signature: Vec<u8> = Vec::new();
        let mut hash = Sha256::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .unwrap_or_else(|error| panic!("{}", update_network_error(&error)))
        {
            hash.update(&chunk);
            signature.extend(chunk.iter().copied().take(2 - signature.len()));
            downloaded += chunk.len() as u64;
        }
        assert_eq!(signature, b"MZ", "installer has no EXE signature");
        assert!(downloaded > 0, "empty installer");
        if let Some(length) = expected_length {
            assert_eq!(downloaded, length, "incomplete installer");
        }
        let expected = validate_update_sha256(
            std::env::var("YT_DLP_GUI_UPDATE_TEST_SHA256")
                .ok()
                .as_deref(),
        )
        .unwrap();
        verify_update_sha256(hash, &expected).unwrap();
        println!("GitHub installer stream and SHA-256 verified: {downloaded} bytes");
    }

    #[test]
    fn update_urls_are_restricted_to_this_repository() {
        assert!(validate_update_url("https://github.com/ssk-shandm/GrabMeta/releases/download/v2.0.2/app_x64-setup.exe").is_ok());
        assert!(validate_update_url("https://github.com/ssk-shandm/GrabMeta/releases/download/v2.3.0/GrabMeta_2.3.0_x64-setup.exe").is_ok());
        for url in [
            "http://github.com/ssk-shandm/GrabMeta/releases/download/v2.0.2/app.exe",
            "https://github.com/other/repo/releases/download/v2.0.2/app.exe",
            "https://github.com/ssk-shandm/GrabMeta/releases/latest",
            "https://github.com.evil.example/ssk-shandm/GrabMeta/releases/download/v2/app.exe",
            "https://user@github.com/ssk-shandm/GrabMeta/releases/download/v2/app.exe",
            "https://github.com:8443/ssk-shandm/GrabMeta/releases/download/v2/app.exe",
            "https://github.com/ssk-shandm/GrabMeta/releases/download/v2/app.exe?redirect=evil",
            "https://github.com/ssk-shandm/GrabMeta/releases/download/v2/app.exe#fragment",
            "https://release-assets.githubusercontent.com/asset.exe",
        ] {
            assert!(validate_update_url(url).is_err(), "{url}");
        }
    }

    #[test]
    fn update_digest_is_required_and_checked() {
        let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
        assert_eq!(
            validate_update_sha256(Some(&expected.to_uppercase())).unwrap(),
            expected
        );
        assert_eq!(
            validate_update_sha256(None).unwrap_err().code,
            "update.sha256Missing"
        );
        for value in [
            Some(""),
            Some("sha256:bad"),
            Some("xyz"),
            Some(&"g".repeat(64)),
        ] {
            assert!(validate_update_sha256(value).is_err());
        }
        let mut hash = Sha256::new();
        hash.update(b"a");
        hash.update(b"bc");
        assert!(verify_update_sha256(hash.clone(), expected).is_ok());
        assert!(verify_update_sha256(hash, &"0".repeat(64)).is_err());
    }

    #[test]
    fn failed_download_cleanup_removes_only_the_partial_file() {
        let root = std::env::temp_dir().join(format!(
            "yt-dlp-gui-update-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&root).unwrap();
        let partial = root.join("installer.exe.part");
        let target = root.join("installer.exe");
        fs::write(&target, b"existing installer").unwrap();
        {
            let _guard = PartialDownload(partial.clone());
            fs::write(&partial, b"incomplete download").unwrap();
        }
        assert!(!partial.exists());
        assert_eq!(fs::read(&target).unwrap(), b"existing installer");
        fs::remove_file(target).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn network_errors_keep_the_underlying_cause() {
        #[derive(Debug)]
        struct Failure(Option<Box<Failure>>, &'static str);

        impl std::fmt::Display for Failure {
            fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.1)
            }
        }

        impl Error for Failure {
            fn source(&self) -> Option<&(dyn Error + 'static)> {
                self.0.as_deref().map(|error| error as &dyn Error)
            }
        }

        let error = Failure(
            Some(Box::new(Failure(
                None,
                "invalid peer certificate: UnknownIssuer",
            ))),
            "error sending request",
        );
        assert_eq!(
            error_chain(&error),
            "error sending request → invalid peer certificate: UnknownIssuer"
        );
    }

    #[test]
    fn updater_client_can_be_built_with_system_trust() {
        assert!(update_http_client(None).is_ok());
    }

    #[test]
    fn update_operations_are_serialized() {
        let state = UpdateState::default();
        let busy = state.begin().unwrap();
        assert_eq!(state.begin().err().map(|error| error.code), Some("update.busy"));
        drop(busy);
        assert!(state.begin().is_ok());
    }

    #[test]
    fn nonproduction_identity_cannot_launch_production_updates() {
        assert!(validate_update_identity("com.ssk-shandm.grabmeta").is_ok());
        assert!(validate_update_identity("com.ssk-shandm.grabmeta.install-test").is_err());
        assert!(validate_update_identity("com.ssk-shandm.ytdlp-gui").is_err());
        assert!(validate_update_identity("another.app").is_err());
    }

    #[test]
    fn installer_names_are_restricted_to_nsis_packages() {
        assert!(safe_update_file_name("GrabMeta_1.2.0_x64-setup.exe").is_ok());
        for name in ["yt-dlp.exe", "../app-setup.exe", "app-setup.zip"] {
            assert!(safe_update_file_name(name).is_err());
        }
    }
}
