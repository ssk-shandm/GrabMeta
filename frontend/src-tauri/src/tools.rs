use crate::{
    model::{AppError, AppResult},
    process::tool_directory,
    AppState,
};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Notify;

mod transfer;

const YTDLP_REPOSITORY: &str = "yt-dlp/yt-dlp";
const FFMPEG_REPOSITORY: &str = "BtbN/FFmpeg-Builds";
const FILE_NAMES: [&str; 3] = ["yt-dlp.exe", "ffmpeg.exe", "ffprobe.exe"];
const REDIRECT_REJECTED: &str = "redirect rejected";
// Backup directory used to swap `bin/` atomically. Pre-2.3.3 builds used the old
// project name; keep reading it so an install interrupted before the rename can
// still be recovered instead of losing the tool directory.
const BACKUP_NAME: &str = ".grabmeta-bin-backup";
const LEGACY_BACKUP_NAME: &str = ".ytdlp-gui-bin-backup";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ToolProfile {
    Basic,
    Full,
}
impl ToolProfile {
    fn asset_name(self) -> &'static str {
        match self {
            Self::Basic => "ffmpeg-master-latest-win64-lgpl.zip",
            Self::Full => "ffmpeg-master-latest-win64-gpl.zip",
        }
    }
}

#[derive(Default)]
pub struct ToolInstallState {
    busy: AtomicBool,
    cancelled: Arc<AtomicBool>,
    notify: Notify,
}
struct Busy<'a>(&'a ToolInstallState);
impl Drop for Busy<'_> {
    fn drop(&mut self) {
        self.0.busy.store(false, Ordering::SeqCst);
    }
}
impl ToolInstallState {
    fn begin(&self) -> AppResult<Busy<'_>> {
        self.busy
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| AppError::new("tools.installing"))?;
        self.cancelled.store(false, Ordering::SeqCst);
        Ok(Busy(self))
    }
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
        self.notify.notify_one();
    }
    fn check(&self) -> AppResult<()> {
        check_cancelled(&self.cancelled)
    }
    async fn wait<T>(
        &self,
        future: impl std::future::Future<Output = AppResult<T>>,
    ) -> AppResult<T> {
        self.check()?;
        tokio::select! {
            biased;
            _ = async { loop {
                let notified = self.notify.notified();
                tokio::pin!(notified);
                notified.as_mut().enable();
                if self.cancelled.load(Ordering::SeqCst) { break; }
                notified.await;
            } } => Err(AppError::new("tools.cancelled")),
            result = future => { self.check()?; result }
        }
    }
}
fn check_cancelled(cancelled: &AtomicBool) -> AppResult<()> {
    if cancelled.load(Ordering::SeqCst) {
        Err(AppError::new("tools.cancelled"))
    } else {
        Ok(())
    }
}
struct TaskPause<'a>(&'a crate::process::TaskRegistry);
impl Drop for TaskPause<'_> {
    fn drop(&mut self) {
        self.0.end_tools_install();
    }
}
struct Staging(PathBuf);
impl Drop for Staging {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolStatus {
    pub installed: bool,
    pub profile: Option<ToolProfile>,
    pub bin_path: String,
    pub ytdlp_version: Option<String>,
    pub ffmpeg_version: Option<String>,
    pub ffprobe_version: Option<String>,
    pub missing: Vec<String>,
    pub error: Option<AppError>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress {
    profile: ToolProfile,
    stage: &'static str,
    downloaded: u64,
    total: Option<u64>,
    percent: Option<u8>,
}
fn emit_progress(
    app: &AppHandle,
    profile: ToolProfile,
    stage: &'static str,
    downloaded: u64,
    total: Option<u64>,
) {
    let percent = total.filter(|n| *n > 0).map(|n| {
        downloaded
            .saturating_mul(100)
            .checked_div(n)
            .unwrap_or(0)
            .min(100) as u8
    });
    let _ = app.emit(
        "tool-download-progress",
        Progress {
            profile,
            stage,
            downloaded,
            total,
            percent,
        },
    );
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Asset {
    repository: String,
    release_tag: String,
    published_at: String,
    name: String,
    url: String,
    size: u64,
    sha256: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ToolMarker {
    schema_version: u8,
    profile: ToolProfile,
    installed_at_unix: u64,
    ytdlp: Asset,
    ffmpeg: Asset,
    versions: Vec<String>,
    ytdlp_notices: Asset,
}
fn marker_path(dir: &Path) -> PathBuf {
    dir.join("tool-profile.json")
}
fn read_marker(dir: &Path) -> Option<ToolMarker> {
    serde_json::from_slice::<ToolMarker>(&fs::read(marker_path(dir)).ok()?)
        .ok()
        .filter(|m| m.schema_version == 1)
}

fn validate_download_url(value: &str) -> AppResult<()> {
    let url = url::Url::parse(value).map_err(|_| AppError::new("tools.downloadUrlInvalid"))?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
    {
        return Err(AppError::new("tools.downloadUrlNotHttps"));
    }
    if !matches!(
        url.host_str(),
        Some(
            "api.github.com"
                | "github.com"
                | "objects.githubusercontent.com"
                | "release-assets.githubusercontent.com"
                | "github-releases.githubusercontent.com"
        )
    ) {
        return Err(AppError::new("tools.downloadHostUnsupported"));
    }
    Ok(())
}
fn client(settings: &crate::model::Settings) -> AppResult<reqwest::Client> {
    crate::model::configure_http_proxy(reqwest::Client::builder(), settings)?
        // Range workers need independent TCP connections. HTTP/2 multiplexing
        // can otherwise put all four requests on one throttled connection.
        .http1_only()
        .user_agent("grabmeta-tools")
        .https_only(true)
        .connect_timeout(Duration::from_secs(30))
        .read_timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 5 {
                attempt.error(REDIRECT_REJECTED)
            } else if validate_download_url(attempt.url().as_str()).is_err() {
                attempt.error(REDIRECT_REJECTED)
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| network_error("tools.clientInit", &e))
}
fn io_error(error: std::io::Error) -> AppError {
    AppError::with("tools.ioFailed", error.to_string())
}
fn network_error(code: &'static str, error: &reqwest::Error) -> AppError {
    use std::error::Error;
    if error.is_redirect() {
        return AppError::new("tools.redirectRejected");
    }
    let mut detail = error.to_string();
    let mut source = error.source();
    while let Some(cause) = source {
        detail.push_str(&format!(" → {cause}"));
        source = cause.source();
    }
    AppError::with(code, detail)
}
fn asset_from_metadata(
    repository: &str,
    name: &str,
    metadata: &serde_json::Value,
) -> AppResult<Asset> {
    let asset = metadata["assets"]
        .as_array()
        .and_then(|list| list.iter().find(|a| a["name"].as_str() == Some(name)))
        .ok_or_else(|| AppError::with("tools.assetMissing", name))?;
    let url = asset["browser_download_url"]
        .as_str()
        .ok_or_else(|| AppError::new("tools.downloadUrlMissing"))?;
    validate_download_url(url)?;
    let prefix = format!("https://github.com/{repository}/releases/download/");
    if !url.starts_with(&prefix) {
        return Err(AppError::new("tools.assetNotProject"));
    }
    let sha256 = asset["digest"]
        .as_str()
        .and_then(|s| s.strip_prefix("sha256:"))
        .filter(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| AppError::new("tools.digestMissing"))?;
    let size = asset["size"]
        .as_u64()
        .filter(|size| *size > 0 && *size <= 512 * 1024 * 1024)
        .ok_or_else(|| AppError::new("tools.assetSizeInvalid"))?;
    Ok(Asset {
        repository: repository.into(),
        release_tag: metadata["tag_name"]
            .as_str()
            .ok_or_else(|| AppError::new("tools.releaseTagMissing"))?
            .into(),
        published_at: metadata["published_at"].as_str().unwrap_or_default().into(),
        name: name.into(),
        url: url.into(),
        size,
        sha256: sha256.to_ascii_lowercase(),
    })
}
fn ffmpeg_asset_name(profile: ToolProfile, metadata: &serde_json::Value) -> AppResult<&str> {
    let assets = metadata["assets"]
        .as_array()
        .ok_or_else(|| AppError::new("tools.assetListMissing"))?;
    // The rolling `latest` tag has aliases, but /releases/latest normally returns
    // an immutable daily release with versioned master builds. Never select a
    // shared, nonfree, other-architecture or stable-branch archive by accident.
    let suffix = match profile {
        ToolProfile::Basic => "-win64-lgpl.zip",
        ToolProfile::Full => "-win64-gpl.zip",
    };
    let candidates: Vec<&str> = assets
        .iter()
        .filter_map(|asset| asset["name"].as_str())
        .filter(|name| {
            if *name == profile.asset_name() {
                return true;
            }
            name.strip_prefix("ffmpeg-N-")
                .and_then(|name| name.strip_suffix(suffix))
                .is_some_and(|version| {
                    version.split_once("-g").is_some_and(|(count, revision)| {
                        !count.is_empty()
                            && count.bytes().all(|c| c.is_ascii_digit())
                            && (7..=40).contains(&revision.len())
                            && revision.bytes().all(|c| c.is_ascii_hexdigit())
                    })
                })
        })
        .collect();
    match candidates.as_slice() {
        [name] => Ok(name),
        [] => Err(AppError::with(
            "tools.ffmpegBuildMissing",
            format!("{profile:?}"),
        )),
        _ => Err(AppError::new("tools.ffmpegBuildAmbiguous")),
    }
}
async fn resolve_release(
    client: &reqwest::Client,
    state: &ToolInstallState,
    repository: &str,
) -> AppResult<serde_json::Value> {
    let url = format!("https://api.github.com/repos/{repository}/releases/latest");
    let mut response = state
        .wait(async {
            client
                .get(&url)
                .send()
                .await
                .map_err(|e| network_error("tools.upstreamUnreachable", &e))
        })
        .await?;
    if !response.status().is_success() {
        return Err(AppError::with(
            "tools.upstreamHttp",
            response.status().as_u16().to_string(),
        ));
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = state
        .wait(async {
            response
                .chunk()
                .await
                .map_err(|e| network_error("tools.upstreamUnreachable", &e))
        })
        .await?
    {
        if bytes.len() + chunk.len() > 2 * 1024 * 1024 {
            return Err(AppError::new("tools.upstreamTooLarge"));
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes)
        .map_err(|e| AppError::with("tools.upstreamInvalid", e.to_string()))
}
async fn resolve_asset(
    client: &reqwest::Client,
    state: &ToolInstallState,
    repository: &str,
    name: &str,
) -> AppResult<Asset> {
    let metadata = resolve_release(client, state, repository).await?;
    let name = if repository == FFMPEG_REPOSITORY {
        let profile = if name == ToolProfile::Basic.asset_name() {
            ToolProfile::Basic
        } else if name == ToolProfile::Full.asset_name() {
            ToolProfile::Full
        } else {
            return Err(AppError::new("tools.ffmpegConfigUnsupported"));
        };
        ffmpeg_asset_name(profile, &metadata)?
    } else {
        name
    };
    asset_from_metadata(repository, name, &metadata)
}
async fn download_file(
    app: &AppHandle,
    profile: ToolProfile,
    client: &reqwest::Client,
    state: &ToolInstallState,
    asset: &Asset,
    target: &Path,
) -> AppResult<()> {
    validate_download_url(&asset.url)?;
    transfer::download(client, state, asset, target, |downloaded| {
        emit_progress(
            app,
            profile,
            "tools.stage.download",
            downloaded,
            Some(asset.size),
        );
    })
    .await
}

fn is_safe_archive_name(name: &str) -> bool {
    // Use Windows rules even when unit tests run on another OS.
    !name.is_empty()
        && !name.starts_with(['/', '\\'])
        && !name.contains([':', '\0'])
        && !name
            .replace('\\', "/")
            .split('/')
            .any(|part| part == ".." || part.ends_with(' ') || part.ends_with('.'))
}
fn validate_pe(path: &Path) -> AppResult<()> {
    let mut file = File::open(path)
        .map_err(|e| AppError::with("tools.fileRead", format!("{}: {e}", path.display())))?;
    let mut header = [0u8; 64];
    file.read_exact(&mut header)
        .map_err(|_| AppError::new("tools.fileIncomplete"))?;
    if &header[..2] != b"MZ" {
        return Err(AppError::new("tools.notExe"));
    }
    use std::io::{Seek, SeekFrom};
    let offset = u32::from_le_bytes(header[60..64].try_into().unwrap()) as u64;
    file.seek(SeekFrom::Start(offset)).map_err(io_error)?;
    let mut signature = [0u8; 6];
    file.read_exact(&mut signature)
        .map_err(|_| AppError::new("tools.peHeaderIncomplete"))?;
    if &signature[..4] != b"PE\0\0" || signature[4..6] != [0x64, 0x86] {
        return Err(AppError::new("tools.notX64Exe"));
    }
    Ok(())
}
fn archive_error(error: zip::result::ZipError) -> AppError {
    AppError::with("tools.archiveCorrupt", error.to_string())
}
fn extract_ffmpeg(zip_path: &Path, dir: &Path, cancelled: &AtomicBool) -> AppResult<()> {
    let mut archive =
        zip::ZipArchive::new(File::open(zip_path).map_err(io_error)?).map_err(archive_error)?;
    if archive.len() > 20_000 {
        return Err(AppError::new("tools.archiveTooManyEntries"));
    }
    let mut found = [false, false];
    let materials = dir.join("licenses/ffmpeg");
    fs::create_dir_all(&materials).map_err(io_error)?;
    let mut expanded = 0u64;
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        let name = entry.name().to_string();
        if !is_safe_archive_name(&name)
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(AppError::new("tools.archiveUnsafePath"));
        }
        if entry.is_dir() {
            continue;
        }
        let normalized = name.replace('\\', "/");
        let leaf = normalized
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let (target, max_size) = match leaf.as_str() {
            "ffmpeg.exe" | "ffprobe.exe" => {
                let i = usize::from(leaf == "ffprobe.exe");
                if found[i] {
                    return Err(AppError::new("tools.archiveDuplicate"));
                }
                found[i] = true;
                (dir.join(&leaf), 512 * 1024 * 1024)
            }
            "license.txt" | "readme.txt" | "readme.md" | "copying.lgplv2.1" | "copying.gplv3" => {
                (materials.join(&leaf), 4 * 1024 * 1024)
            }
            _ => continue,
        };
        if entry.size() > max_size {
            return Err(AppError::new("tools.extractTooLarge"));
        }
        let mut output = File::create(&target).map_err(io_error)?;
        let mut buffer = [0u8; 65536];
        let mut written = 0u64;
        loop {
            check_cancelled(cancelled)?;
            let n = entry
                .read(&mut buffer)
                .map_err(|e| AppError::with("tools.extractFailed", e.to_string()))?;
            if n == 0 {
                break;
            }
            written += n as u64;
            expanded += n as u64;
            if written > max_size || expanded > 1100 * 1024 * 1024 {
                return Err(AppError::new("tools.extractTooLarge"));
            }
            output.write_all(&buffer[..n]).map_err(io_error)?;
        }
        output.sync_all().map_err(io_error)?;
    }
    if !found.into_iter().all(|v| v) {
        return Err(AppError::new("tools.archiveMissingTools"));
    }
    for name in ["ffmpeg.exe", "ffprobe.exe"] {
        validate_pe(&dir.join(name))?;
    }
    Ok(())
}

fn extract_ytdlp_notices(zip_path: &Path, dir: &Path, cancelled: &AtomicBool) -> AppResult<()> {
    let mut archive =
        zip::ZipArchive::new(File::open(zip_path).map_err(io_error)?).map_err(archive_error)?;
    if archive.len() > 20_000 {
        return Err(AppError::new("tools.archiveTooManyEntries"));
    }
    let materials = dir.join("licenses/yt-dlp");
    fs::create_dir_all(&materials).map_err(io_error)?;
    let mut found = false;
    let mut total = 0u64;
    for index in 0..archive.len() {
        check_cancelled(cancelled)?;
        let mut entry = archive.by_index(index).map_err(archive_error)?;
        let name = entry.name().replace('\\', "/");
        if !is_safe_archive_name(&name)
            || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err(AppError::new("tools.archiveUnsafePath"));
        }
        if entry.is_dir() {
            continue;
        }
        let leaf = name
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let target = if leaf == "third_party_licenses.txt" {
            if found {
                return Err(AppError::new("tools.archiveDuplicate"));
            }
            found = true;
            materials.join("THIRD_PARTY_LICENSES.txt")
        } else if matches!(
            leaf.as_str(),
            "license" | "license.txt" | "notice" | "notice.txt" | "copying"
        ) {
            materials.join(format!("dependency-{index}-notice.txt"))
        } else {
            continue;
        };
        if entry.size() > 4 * 1024 * 1024 {
            return Err(AppError::new("tools.extractTooLarge"));
        }
        let mut bytes = Vec::new();
        (&mut entry)
            .take(4 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(io_error)?;
        total += bytes.len() as u64;
        if bytes.len() > 4 * 1024 * 1024 || total > 16 * 1024 * 1024 {
            return Err(AppError::new("tools.extractTooLarge"));
        }
        fs::write(target, bytes).map_err(io_error)?;
    }
    if !found {
        return Err(AppError::new("tools.noticesMissing"));
    }
    fs::write(
        materials.join("UNLICENSE.txt"),
        include_str!("../../../licenses/yt-dlp/UNLICENSE.txt"),
    )
    .map_err(io_error)?;
    Ok(())
}

async fn version(dir: &Path, name: &str, argument: &str) -> AppResult<String> {
    let mut command = tokio::process::Command::new(dir.join(name));
    command
        .arg(argument)
        .kill_on_drop(true)
        .stdin(std::process::Stdio::null());
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    let output = tokio::time::timeout(Duration::from_secs(15), command.output())
        .await
        .map_err(|_| AppError::with("tools.versionTimeout", name))?
        .map_err(|e| AppError::with("tools.versionStart", format!("{name}: {e}")))?;
    if !output.status.success() {
        return Err(AppError::with("tools.versionFailed", name));
    }
    let text = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    if text.is_empty() {
        return Err(AppError::with("tools.versionEmpty", name));
    }
    Ok(text)
}
async fn probe_versions(dir: &Path) -> AppResult<Vec<String>> {
    for name in FILE_NAMES {
        validate_pe(&dir.join(name))?;
    }
    let (a, b, c) = tokio::try_join!(
        version(dir, "yt-dlp.exe", "--version"),
        version(dir, "ffmpeg.exe", "-version"),
        version(dir, "ffprobe.exe", "-version")
    )?;
    Ok(vec![a, b, c])
}
async fn status_at(dir: PathBuf) -> ToolStatus {
    let missing = FILE_NAMES
        .iter()
        .filter(|n| !dir.join(n).is_file())
        .map(|n| (*n).into())
        .collect::<Vec<_>>();
    let mut result = ToolStatus {
        installed: false,
        profile: None,
        bin_path: dir.to_string_lossy().into_owned(),
        ytdlp_version: None,
        ffmpeg_version: None,
        ffprobe_version: None,
        missing,
        error: None,
    };
    if !result.missing.is_empty() {
        return result;
    }
    match probe_versions(&dir).await {
        Ok(versions) => {
            result.installed = true;
            result.ytdlp_version = Some(versions[0].clone());
            result.ffmpeg_version = Some(versions[1].clone());
            result.ffprobe_version = Some(versions[2].clone());
            result.profile = read_marker(&dir)
                .filter(|m| m.versions == versions)
                .map(|m| m.profile);
        }
        Err(error) => result.error = Some(error),
    }
    result
}
// Windows reports transient scanner or indexer locks on a freshly written directory as access denied.
fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut attempts = 0;
    loop {
        match fs::rename(from, to) {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied && attempts < 10 => {
                attempts += 1;
                std::thread::sleep(Duration::from_millis(300));
            }
            result => return result,
        }
    }
}
fn replace_install(stage: &Path, target: &Path) -> AppResult<()> {
    for name in FILE_NAMES.into_iter().chain(["tool-profile.json"]) {
        if !stage.join(name).is_file() {
            return Err(AppError::with("tools.fileMissing", name));
        }
    }
    let parent = target
        .parent()
        .ok_or_else(|| AppError::new("tools.directoryUnknown"))?;
    let backup = parent.join(BACKUP_NAME);
    let legacy_backup = parent.join(LEGACY_BACKUP_NAME);
    if backup.exists() || legacy_backup.exists() {
        return Err(AppError::with(
            "tools.backupLeftover",
            backup.exists().then(|| backup.display().to_string())
                .unwrap_or_else(|| legacy_backup.display().to_string()),
        ));
    }
    if target.exists() {
        if target.is_symlink() || !target.is_dir() {
            return Err(AppError::new("tools.targetNotDirectory"));
        }
        fs::rename(target, &backup).map_err(|e| {
            AppError::with(
                "tools.replacePrepareFailed",
                format!("{e}; from {} to {}", target.display(), backup.display()),
            )
        })?;
    }
    if let Err(error) = rename_with_retry(stage, target) {
        if backup.exists() {
            fs::rename(&backup, target).map_err(|rollback| {
                AppError::with(
                    "tools.rollbackFailed",
                    format!("{error}; {rollback}; {}", backup.display()),
                )
            })?;
        }
        return Err(AppError::with(
            "tools.installKeptOld",
            format!("{error}; from {} to {}", stage.display(), target.display()),
        ));
    }
    // A failed backup cleanup must not falsely report an already committed install as failed.
    if backup.exists() {
        let _ = fs::remove_dir_all(&backup);
    }
    Ok(())
}
fn recover_backup(target: &Path) -> AppResult<()> {
    let parent = target
        .parent()
        .ok_or_else(|| AppError::new("tools.directoryUnknown"))?;
    // Prefer the current name, but fall back to a backup left by a pre-2.3.3 build.
    let backup = [parent.join(BACKUP_NAME), parent.join(LEGACY_BACKUP_NAME)]
        .into_iter()
        .find(|path| {
            !target.exists()
                && path.is_dir()
                && FILE_NAMES.iter().all(|n| path.join(n).is_file())
        });
    if let Some(backup) = backup {
        fs::rename(&backup, target)
            .map_err(|e| AppError::with("tools.recoverFailed", e.to_string()))?;
    }
    Ok(())
}
#[tauri::command]
pub async fn get_tool_status(app: AppHandle) -> AppResult<ToolStatus> {
    let dir = tool_directory(&app)?;
    if app.state::<ToolInstallState>().busy.load(Ordering::SeqCst) {
        return Err(AppError::new("tools.installing"));
    }
    recover_backup(&dir)?;
    Ok(status_at(dir).await)
}
#[tauri::command]
pub fn cancel_tool_download(state: State<'_, ToolInstallState>) {
    if state.busy.load(Ordering::SeqCst) {
        state.cancel();
    }
}

#[tauri::command]
pub async fn download_tools(
    app: AppHandle,
    state: State<'_, ToolInstallState>,
    profile: ToolProfile,
) -> AppResult<ToolStatus> {
    let _busy = state.begin()?;
    let app_state = app.state::<AppState>();
    app_state.tasks.begin_tools_install()?;
    let _paused = TaskPause(&app_state.tasks);
    let dir = tool_directory(&app)?;
    let parent = dir
        .parent()
        .ok_or_else(|| AppError::new("tools.directoryUnknown"))?;
    fs::create_dir_all(parent)
        .map_err(|e| AppError::with("tools.directoryCreate", e.to_string()))?;
    let parent = fs::canonicalize(parent).map_err(io_error)?;
    let target = parent.join("bin");
    recover_backup(&target)?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::with("tools.ioFailed", e.to_string()))?;
    let staging = Staging(parent.join(format!(
        ".grabmeta-install-{}-{}",
        std::process::id(),
        stamp.as_nanos()
    )));
    fs::create_dir(&staging.0).map_err(|e| AppError::with("tools.stagingCreate", e.to_string()))?;
    emit_progress(&app, profile, "tools.stage.readUpstream", 0, None);
    let settings = app
        .state::<AppState>()
        .settings
        .lock()
        .map_err(|_| AppError::new("tools.settingsLocked"))?
        .clone();
    let client = client(&settings)?;
    // EXE and notices must come from one immutable release snapshot, not two
    // requests that can straddle a release or duplicate slow API calls.
    let release = resolve_release(&client, &state, YTDLP_REPOSITORY).await?;
    let ytdlp = asset_from_metadata(YTDLP_REPOSITORY, "yt-dlp.exe", &release)?;
    let ytdlp_notices = asset_from_metadata(YTDLP_REPOSITORY, "yt-dlp_win.zip", &release)?;
    let ffmpeg = resolve_asset(&client, &state, FFMPEG_REPOSITORY, profile.asset_name()).await?;
    download_file(
        &app,
        profile,
        &client,
        &state,
        &ytdlp,
        &staging.0.join("yt-dlp.exe"),
    )
    .await?;
    download_file(
        &app,
        profile,
        &client,
        &state,
        &ffmpeg,
        &staging.0.join("ffmpeg.zip"),
    )
    .await?;
    download_file(
        &app,
        profile,
        &client,
        &state,
        &ytdlp_notices,
        &staging.0.join("yt-dlp-notices.zip"),
    )
    .await?;
    emit_progress(&app, profile, "tools.stage.extract", 0, None);
    let extract_dir = staging.0.clone();
    let cancelled = state.cancelled.clone();
    // Always join extraction before dropping staging; cancellation cannot leave a writer behind.
    tokio::task::spawn_blocking(move || {
        extract_ffmpeg(&extract_dir.join("ffmpeg.zip"), &extract_dir, &cancelled)?;
        extract_ytdlp_notices(
            &extract_dir.join("yt-dlp-notices.zip"),
            &extract_dir,
            &cancelled,
        )
    })
    .await
    .map_err(|e| AppError::with("tools.extractFailed", e.to_string()))??;
    fs::remove_file(staging.0.join("ffmpeg.zip")).map_err(io_error)?;
    fs::remove_file(staging.0.join("yt-dlp-notices.zip")).map_err(io_error)?;
    emit_progress(&app, profile, "tools.stage.verify", 0, None);
    let versions = state.wait(probe_versions(&staging.0)).await?;
    for (name, args) in [
        ("license-output.txt", vec!["-L"]),
        ("buildconf-output.txt", vec!["-buildconf"]),
    ] {
        let mut command = tokio::process::Command::new(staging.0.join("ffmpeg.exe"));
        command
            .args(args)
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null());
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let output = state
            .wait(async {
                tokio::time::timeout(Duration::from_secs(15), command.output())
                    .await
                    .map_err(|_| AppError::new("tools.buildInfoTimeout"))?
                    .map_err(|e| AppError::with("tools.buildInfoFailed", e.to_string()))
            })
            .await?;
        if !output.status.success() {
            return Err(AppError::new("tools.buildInfoFailed"));
        }
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if name == "buildconf-output.txt" {
            let gpl = text.contains("--enable-gpl");
            if text.contains("--enable-nonfree") || gpl != (profile == ToolProfile::Full) {
                return Err(AppError::new("tools.ffmpegLicenseMismatch"));
            }
        }
        fs::write(staging.0.join("licenses/ffmpeg").join(name), text).map_err(io_error)?;
    }
    let marker = ToolMarker {
        schema_version: 1,
        profile,
        installed_at_unix: stamp.as_secs(),
        ytdlp,
        ffmpeg,
        ytdlp_notices,
        versions: versions.clone(),
    };
    let marker_bytes = serde_json::to_vec_pretty(&marker)
        .map_err(|e| AppError::with("tools.ioFailed", e.to_string()))?;
    fs::write(marker_path(&staging.0), &marker_bytes).map_err(io_error)?;
    fs::write(
        staging.0.join("licenses/installed-tools.json"),
        marker_bytes,
    )
    .map_err(io_error)?;
    fs::write(staging.0.join("licenses/NOTICE.txt"), format!("Tools downloaded directly by this application from upstream GitHub Releases.\nProfile: {profile:?}; FFmpeg provider: BtbN/FFmpeg-Builds.\nActual versions and SHA-256 verification records: installed-tools.json.\nFFmpeg license and build configuration: ffmpeg/license-output.txt and ffmpeg/buildconf-output.txt.\nUpstream: https://github.com/yt-dlp/yt-dlp and https://github.com/BtbN/FFmpeg-Builds.\nSource leads: https://github.com/yt-dlp/yt-dlp/tree/{} and https://github.com/BtbN/FFmpeg-Builds/tree/master/scripts.d.\nThese source leads are NOT a complete Corresponding Source or written source offer.\nDo not redistribute these binaries before a separate licensing review.\n", marker.ytdlp.release_tag)).map_err(io_error)?;
    state.check()?;
    emit_progress(&app, profile, "tools.stage.install", 0, None);
    // No await between final cancellation check and directory transaction.
    replace_install(&staging.0, &target)?;
    emit_progress(&app, profile, "tools.stage.done", 1, Some(1));
    Ok(ToolStatus {
        installed: true,
        profile: Some(profile),
        bin_path: target.to_string_lossy().into_owned(),
        ytdlp_version: Some(versions[0].clone()),
        ffmpeg_version: Some(versions[1].clone()),
        ffprobe_version: Some(versions[2].clone()),
        missing: vec![],
        error: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_require_https_exact_hosts_and_no_credentials() {
        for url in [
            "https://github.com/yt-dlp/yt-dlp/releases/download/1/yt-dlp.exe",
            "https://release-assets.githubusercontent.com/a",
        ] {
            assert!(validate_download_url(url).is_ok());
        }
        for url in [
            "http://github.com/a",
            "https://evilgithub.com/a",
            "https://github.com.evil/a",
            "https://user@github.com/a",
            "https://github.com:444/a",
            "file:///a",
        ] {
            assert!(validate_download_url(url).is_err(), "{url}");
        }
    }
    #[test]
    fn profiles_are_strict_and_map_to_static_archives() {
        assert_eq!(
            serde_json::from_str::<ToolProfile>("\"basic\"").unwrap(),
            ToolProfile::Basic
        );
        assert!(serde_json::from_str::<ToolProfile>("\"nonfree\"").is_err());
        assert!(ToolProfile::Basic.asset_name().ends_with("-lgpl.zip"));
        assert!(ToolProfile::Full.asset_name().ends_with("-gpl.zip"));
    }
    #[test]
    fn paths_use_windows_security_rules() {
        assert!(is_safe_archive_name("root/bin/ffmpeg.exe"));
        for path in [
            "../ffmpeg.exe",
            "root/../../ffprobe.exe",
            "C:/ffmpeg.exe",
            "\\\\host\\a",
            "/a",
            "root\\..\\a",
            "a.exe:stream",
            "root /file",
            "root./file",
        ] {
            assert!(!is_safe_archive_name(path), "{path}");
        }
    }
    #[test]
    fn installs_are_serialized_and_can_retry_after_cancellation() {
        let state = ToolInstallState::default();
        let busy = state.begin().unwrap();
        assert!(state.begin().is_err());
        state.cancel();
        assert!(state.check().is_err());
        drop(busy);
        let _busy = state.begin().unwrap();
        assert!(state.check().is_ok());
    }
    #[test]
    fn metadata_requires_digest_and_expected_repository() {
        let mut metadata = serde_json::json!({"tag_name":"1", "published_at":"x", "assets":[{"name":"yt-dlp.exe", "size":100, "digest":format!("sha256:{}", "a".repeat(64)), "browser_download_url":"https://github.com/yt-dlp/yt-dlp/releases/download/1/yt-dlp.exe"}]});
        assert!(asset_from_metadata(YTDLP_REPOSITORY, "yt-dlp.exe", &metadata).is_ok());
        assert!(asset_from_metadata(FFMPEG_REPOSITORY, "yt-dlp.exe", &metadata).is_err());
        metadata["assets"][0]["digest"] = serde_json::Value::Null;
        assert!(asset_from_metadata(YTDLP_REPOSITORY, "yt-dlp.exe", &metadata).is_err());
    }
    #[test]
    fn daily_ffmpeg_assets_select_only_unique_x64_static_master_builds() {
        let basic = "ffmpeg-N-127252-ga25ba44c0c-win64-lgpl.zip";
        let full = "ffmpeg-N-127252-ga25ba44c0c-win64-gpl.zip";
        let metadata = serde_json::json!({"assets": [
            {"name": basic}, {"name": full},
            {"name": "ffmpeg-N-127252-ga25ba44c0c-win64-lgpl-shared.zip"},
            {"name": "ffmpeg-N-127252-ga25ba44c0c-win64-gpl-shared.zip"},
            {"name": "ffmpeg-n9.0.2-23-g27b46f0fbc-win64-lgpl-9.0.zip"},
            {"name": "ffmpeg-N-127252-ga25ba44c0c-linux64-lgpl.zip"}
        ]});
        assert_eq!(
            ffmpeg_asset_name(ToolProfile::Basic, &metadata).unwrap(),
            basic
        );
        assert_eq!(
            ffmpeg_asset_name(ToolProfile::Full, &metadata).unwrap(),
            full
        );
        let aliases = serde_json::json!({"assets": [
            {"name": ToolProfile::Basic.asset_name()}, {"name": ToolProfile::Full.asset_name()}
        ]});
        assert_eq!(
            ffmpeg_asset_name(ToolProfile::Basic, &aliases).unwrap(),
            ToolProfile::Basic.asset_name()
        );
        for invalid in [
            "ffmpeg-N-127252-ga25ba44c0c-win64-lgpl-shared.zip",
            "ffmpeg-N-127252-ga25ba44c0c-win64-lgpl-nonfree.zip",
            "ffmpeg-N-127252-ga25ba44c0c-win32-lgpl.zip",
            "ffmpeg-N-not-a-revision-win64-lgpl.zip",
        ] {
            assert!(ffmpeg_asset_name(
                ToolProfile::Basic,
                &serde_json::json!({"assets":[{"name":invalid}]})
            )
            .is_err());
        }
        assert!(ffmpeg_asset_name(
            ToolProfile::Basic,
            &serde_json::json!({"assets":[{"name":basic},{"name":basic}]})
        )
        .is_err());
    }
    fn temporary() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "ytdlp-tools-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&dir).unwrap();
        dir
    }
    fn test_pe() -> Vec<u8> {
        let mut data = vec![0; 70];
        data[..2].copy_from_slice(b"MZ");
        data[60..64].copy_from_slice(&64u32.to_le_bytes());
        data[64..70].copy_from_slice(b"PE\0\0\x64\x86");
        data
    }
    fn archive(dir: &Path, names: &[&str]) -> PathBuf {
        let path = dir.join("test.zip");
        let mut zip = zip::ZipWriter::new(File::create(&path).unwrap());
        for name in names {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&test_pe()).unwrap();
        }
        zip.finish().unwrap();
        path
    }
    #[test]
    fn extraction_rejects_missing_duplicate_and_traversing_entries() {
        let dir = temporary();
        let _cleanup = Staging(dir.clone());
        let cancelled = AtomicBool::new(false);
        for names in [
            vec!["root/ffmpeg.exe"],
            vec!["root/ffmpeg.exe", "../ffprobe.exe"],
            vec!["a/ffmpeg.exe", "b/ffmpeg.exe", "ffprobe.exe"],
        ] {
            assert!(extract_ffmpeg(&archive(&dir, &names), &dir, &cancelled).is_err());
        }
        assert!(extract_ffmpeg(
            &archive(
                &dir,
                &[
                    "root/bin/ffmpeg.exe",
                    "root/bin/ffprobe.exe",
                    "root/bin/ffplay.exe"
                ]
            ),
            &dir,
            &cancelled
        )
        .is_ok());
        assert!(!dir.join("ffplay.exe").exists());
        cancelled.store(true, Ordering::SeqCst);
        assert!(extract_ffmpeg(&dir.join("test.zip"), &dir, &cancelled).is_err());
    }
    #[test]
    fn failed_install_preserves_old_tools_and_staging_is_cleaned() {
        let dir = temporary();
        let _cleanup = Staging(dir.clone());
        let target = dir.join("bin");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("old"), "old").unwrap();
        let stage = Staging(dir.join("stage"));
        fs::create_dir(&stage.0).unwrap();
        assert!(replace_install(&stage.0, &target).is_err());
        assert!(target.join("old").exists());
        let stage_path = stage.0.clone();
        drop(stage);
        assert!(!stage_path.exists());
    }
    #[test]
    fn successful_install_is_a_complete_directory_swap() {
        let dir = temporary();
        let _cleanup = Staging(dir.clone());
        let target = dir.join("bin");
        fs::create_dir(&target).unwrap();
        fs::write(target.join("old"), "old").unwrap();
        let stage = dir.join("stage");
        fs::create_dir(&stage).unwrap();
        for name in FILE_NAMES.into_iter().chain(["tool-profile.json"]) {
            fs::write(stage.join(name), "new").unwrap();
        }
        replace_install(&stage, &target).unwrap();
        assert!(!target.join("old").exists());
        assert!(FILE_NAMES.iter().all(|n| target.join(n).exists()));
        assert!(!dir.join(BACKUP_NAME).exists());
    }
    #[test]
    fn crash_recovery_restores_previous_directory() {
        let dir = temporary();
        let _cleanup = Staging(dir.clone());
        let backup = dir.join(BACKUP_NAME);
        fs::create_dir(&backup).unwrap();
        for name in FILE_NAMES {
            fs::write(backup.join(name), "old").unwrap();
        }
        let target = dir.join("bin");
        recover_backup(&target).unwrap();
        assert!(target.join("yt-dlp.exe").exists());
    }
}

#[tauri::command]
pub fn open_tool_licenses(app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let installed = tool_directory(&app)?.join("licenses");
    let directory = if installed.is_dir() {
        installed
    } else {
        app.path()
            .resource_dir()
            .map_err(|e| AppError::with("tools.resourceDirFailed", e.to_string()))?
            .join("licenses")
    };
    if !directory.is_dir() {
        return Err(AppError::new("tools.licensesMissing"));
    }
    app.opener()
        .open_path(directory.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::with("tools.openFailed", e.to_string()))
}

// A separate fixed resource path: installed tool licenses must not hide GUI notices.
#[tauri::command]
pub fn open_gui_licenses(app: AppHandle) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let directory = app
        .path()
        .resource_dir()
        .map_err(|e| AppError::with("tools.resourceDirFailed", e.to_string()))?
        .join("licenses")
        .join("gui");
    if !directory.is_dir() {
        return Err(AppError::new("tools.guiLicensesMissing"));
    }
    app.opener()
        .open_path(directory.to_string_lossy(), None::<&str>)
        .map_err(|e| AppError::with("tools.openFailed", e.to_string()))
}
