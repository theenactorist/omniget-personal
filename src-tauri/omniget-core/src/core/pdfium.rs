use std::io::Read;
use std::path::{Component, Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::Serialize;

const PDFIUM_DOWNLOAD_BASE: &str =
    "https://github.com/bblanchon/pdfium-binaries/releases/latest/download";

#[derive(Debug, Clone, Serialize)]
pub struct DependencyVariant {
    pub id: String,
    pub label: String,
    pub recommended: bool,
}

pub fn pdfium_lib_filename() -> &'static str {
    if cfg!(target_os = "windows") {
        "pdfium.dll"
    } else if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else {
        "libpdfium.so"
    }
}

fn default_archive_id() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        return "win-x64";
    }
    #[cfg(all(target_os = "windows", target_arch = "x86"))]
    {
        return "win-x86";
    }
    #[cfg(all(target_os = "windows", target_arch = "aarch64"))]
    {
        return "win-arm64";
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        return "mac-x64";
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        return "mac-arm64";
    }
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        return "linux-x64";
    }
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        return "linux-arm64";
    }
    #[allow(unreachable_code)]
    "win-x64"
}

pub fn list_variants() -> Vec<DependencyVariant> {
    let default = default_archive_id();
    let make = |id: &str, label: &str| DependencyVariant {
        id: id.to_string(),
        label: label.to_string(),
        recommended: id == default,
    };

    if cfg!(target_os = "windows") {
        vec![
            make("win-x64", "Windows x64 (64-bit)"),
            make("win-x64-v8", "Windows x64 + V8 (com JS, maior)"),
            make("win-x86", "Windows x86 (32-bit)"),
            make("win-arm64", "Windows ARM64"),
        ]
    } else if cfg!(target_os = "macos") {
        vec![
            make("mac-x64", "macOS x64 (Intel)"),
            make("mac-arm64", "macOS ARM64 (Apple Silicon)"),
            make("mac-univ", "macOS universal"),
        ]
    } else {
        vec![
            make("linux-x64", "Linux x64"),
            make("linux-arm64", "Linux ARM64"),
            make("linux-arm", "Linux ARM (32-bit)"),
            make("linux-musl-x64", "Linux musl x64 (Alpine)"),
            make("linux-musl-arm64", "Linux musl ARM64"),
        ]
    }
}

fn archive_name_for_variant(variant: Option<&str>) -> String {
    let id = variant
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && *s != "auto")
        .unwrap_or(default_archive_id());
    if let Some(rest) = id.strip_prefix("win-x64-v8") {
        let _ = rest;
        return "pdfium-v8-win-x64.tgz".to_string();
    }
    format!("pdfium-{}.tgz", id)
}

fn absolute_normalized(path: &Path) -> Option<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Some(normalized)
}

/// Resolve existing parents too: a future library under a symlinked directory
/// must be rejected before an installer creates it or downloads anything.
fn canonical_with_missing_tail(path: &Path) -> Option<PathBuf> {
    let mut existing = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().ok()?.join(path)
    };
    let mut tail = Vec::new();
    loop {
        match std::fs::canonicalize(&existing) {
            Ok(mut resolved) => {
                for component in tail.iter().rev() {
                    resolved.push(component);
                }
                return absolute_normalized(&resolved);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                tail.push(existing.file_name()?.to_os_string());
                if !existing.pop() {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

fn allowed_pdfium_path(path: &Path, app_data: &Path) -> bool {
    let plugins = app_data.join("plugins");
    let Some(lexical_plugins) = absolute_normalized(&plugins) else {
        return false;
    };
    let Some(lexical_candidate) = absolute_normalized(path) else {
        return false;
    };
    if lexical_candidate.starts_with(&lexical_plugins) {
        return false;
    }
    let Some(canonical_plugins) = canonical_with_missing_tail(&plugins) else {
        return false;
    };
    let Some(canonical_candidate) = canonical_with_missing_tail(path) else {
        return false;
    };
    !canonical_candidate.starts_with(&canonical_plugins)
}

fn require_independent_pdfium(path: &Path) -> anyhow::Result<()> {
    let app_data = crate::core::paths::app_data_dir()
        .ok_or_else(|| anyhow!("could not determine app data dir"))?;
    if !allowed_pdfium_path(path, &app_data) {
        return Err(anyhow!("PDFium helpers from the disabled plugins directory cannot be used. Choose an independent PDFium library."));
    }
    Ok(())
}

fn independent_candidate(path: &Path, app_data: &Path) -> Option<PathBuf> {
    (allowed_pdfium_path(path, app_data) && path.is_file()).then(|| path.to_path_buf())
}

fn managed_dir(app_data: &Path) -> Option<PathBuf> {
    let target = app_data.join("bin").join("pdfium");
    allowed_pdfium_path(&target, app_data).then_some(target)
}

pub fn pdfium_target_dir() -> Option<PathBuf> {
    managed_dir(&crate::core::paths::app_data_dir()?)
}

pub fn pdfium_target_path() -> Option<PathBuf> {
    let path = pdfium_target_dir()?.join(pdfium_lib_filename());
    require_independent_pdfium(&path).ok()?;
    Some(path)
}

pub fn pdfium_version_marker_path() -> Option<PathBuf> {
    let path = pdfium_target_dir()?.join("pdfium.version");
    require_independent_pdfium(&path).ok()?;
    Some(path)
}

/// The PDFium library actually in use, and where it came from: `"custom"` for
/// a file the user pointed at, `"managed"` for the one OmniGet downloaded.
///
/// Issue #222: yt-dlp and FFmpeg have honoured the user's own binary since
/// the feature shipped, through `find_tool_with_source`. PDFium never did —
/// only the managed folder was ever checked — so a linked library was
/// recorded and displayed while the status stayed "missing".
pub fn resolve_with_source() -> Option<(PathBuf, &'static str)> {
    let app_data = crate::core::paths::app_data_dir()?;
    if let Some(custom) = crate::core::binary_overrides::get("PDFium") {
        if let Some(path) = independent_candidate(&custom, &app_data) {
            return Some((path, "custom"));
        }
    }
    let managed = pdfium_target_path()?;
    if let Some(path) = independent_candidate(&managed, &app_data) {
        return Some((path, "managed"));
    }
    None
}

/// Path to the PDFium library to load, honouring a custom override.
pub fn resolve_path() -> Option<PathBuf> {
    resolve_with_source().map(|(p, _)| p)
}

pub fn is_installed() -> bool {
    resolve_with_source().is_some()
}

pub fn read_version_marker() -> Option<String> {
    let p = pdfium_version_marker_path()?;
    let s = std::fs::read_to_string(&p).ok()?;
    let trimmed = s.trim().to_string();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

pub async fn ensure_pdfium() -> anyhow::Result<PathBuf> {
    ensure_pdfium_with_variant(None).await
}

pub async fn ensure_pdfium_with_variant(variant: Option<String>) -> anyhow::Result<PathBuf> {
    let target_dir =
        pdfium_target_dir().ok_or_else(|| anyhow!("could not determine app data dir"))?;
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating pdfium target dir {}", target_dir.display()))?;

    let archive_name = archive_name_for_variant(variant.as_deref());
    let url = format!("{}/{}", PDFIUM_DOWNLOAD_BASE, archive_name);
    let lib_filename = pdfium_lib_filename();
    let target_path = target_dir.join(lib_filename);
    require_independent_pdfium(&target_path)?;

    tracing::info!("Downloading pdfium from {}", url);

    let client = crate::core::http_client::apply_global_proxy(reqwest::Client::builder())
        .timeout(std::time::Duration::from_secs(600))
        .build()?;

    let response = client.get(&url).send().await?;
    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to download pdfium from {}: HTTP {}",
            url,
            response.status()
        ));
    }
    let bytes = response.bytes().await?.to_vec();
    if bytes.len() < 100_000 {
        return Err(anyhow!(
            "Downloaded pdfium archive is too small ({} bytes) — likely an error page",
            bytes.len()
        ));
    }

    let target_path_clone = target_path.clone();
    let target_dir_clone = target_dir.clone();
    let lib_name = lib_filename.to_string();
    let extracted_version =
        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
            extract_pdfium_archive(&bytes, &target_path_clone, &target_dir_clone, &lib_name)
        })
        .await
        .map_err(|e| anyhow!("spawn_blocking failed: {}", e))??;

    if let Some(version_marker) = pdfium_version_marker_path() {
        let base = extracted_version.unwrap_or_else(|| "latest".to_string());
        let value = format!("{} ({})", base, archive_name);
        let _ = std::fs::write(&version_marker, value);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = tokio::task::spawn_blocking({
            let p = target_path.clone();
            move || {
                crate::core::process::std_command("xattr")
                    .args(["-d", "com.apple.quarantine"])
                    .arg(&p)
                    .output()
            }
        })
        .await;
    }

    require_independent_pdfium(&target_path)?;
    Ok(target_path)
}

pub fn set_pdfium_from_path(source: &Path) -> anyhow::Result<PathBuf> {
    require_independent_pdfium(source)?;
    if !source.is_file() {
        return Err(anyhow!("source not a file: {}", source.display()));
    }
    let target_dir =
        pdfium_target_dir().ok_or_else(|| anyhow!("could not determine app data dir"))?;
    std::fs::create_dir_all(&target_dir)
        .with_context(|| format!("creating pdfium target dir {}", target_dir.display()))?;
    let lib_filename = pdfium_lib_filename();
    let target_path = target_dir.join(lib_filename);
    require_independent_pdfium(&target_path)?;

    let tmp = target_dir.join(format!(".{}.tmp", lib_filename));
    require_independent_pdfium(&tmp)?;
    std::fs::copy(source, &tmp)
        .with_context(|| format!("copying {} → {}", source.display(), tmp.display()))?;
    if target_path.exists() {
        let _ = std::fs::remove_file(&target_path);
    }
    std::fs::rename(&tmp, &target_path)
        .with_context(|| format!("renaming temp to {}", target_path.display()))?;

    if let Some(version_marker) = pdfium_version_marker_path() {
        let label = source
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("custom");
        let value = format!("custom ({})", label);
        let _ = std::fs::write(&version_marker, value);
    }
    require_independent_pdfium(&target_path)?;
    Ok(target_path)
}

fn extract_pdfium_archive(
    bytes: &[u8],
    target_path: &std::path::Path,
    target_dir: &std::path::Path,
    lib_filename: &str,
) -> anyhow::Result<Option<String>> {
    require_independent_pdfium(target_dir)?;
    require_independent_pdfium(target_path)?;
    let gz = flate2::read::GzDecoder::new(bytes);
    let mut tar_archive = tar::Archive::new(gz);

    let mut found_lib = false;
    let mut version: Option<String> = None;

    for entry in tar_archive.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_path_buf();
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_string();

        if file_name == lib_filename {
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)?;
            let tmp = target_dir.join(format!(".{}.tmp", lib_filename));
            require_independent_pdfium(&tmp)?;
            std::fs::write(&tmp, &buf)
                .with_context(|| format!("writing temp pdfium lib {}", tmp.display()))?;
            if target_path.exists() {
                let _ = std::fs::remove_file(target_path);
            }
            std::fs::rename(&tmp, target_path)
                .with_context(|| format!("moving pdfium to {}", target_path.display()))?;
            found_lib = true;
        } else if file_name == "VERSION" {
            let mut buf = String::new();
            entry.read_to_string(&mut buf)?;
            version = Some(buf.trim().to_string());
        }
    }

    if !found_lib {
        return Err(anyhow!("{} not found inside pdfium archive", lib_filename));
    }

    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_root() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("omniget-pdfium-policy-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        root
    }

    #[test]
    fn pdfium_rejects_plugin_paths_and_preserves_independent_helpers() {
        let root = test_root();
        let plugin = root.join("plugins/study").join(pdfium_lib_filename());
        let legacy_managed = root.join("plugins/study/data").join(pdfium_lib_filename());
        std::fs::create_dir_all(legacy_managed.parent().unwrap()).unwrap();
        std::fs::write(&plugin, b"not a native library").unwrap();
        std::fs::write(&legacy_managed, b"not a native library").unwrap();
        assert!(independent_candidate(&plugin, &root).is_none());
        assert!(independent_candidate(&legacy_managed, &root).is_none());
        assert!(!allowed_pdfium_path(
            &root.join("bin/../plugins/study/missing-library"),
            &root
        ));

        let managed = managed_dir(&root).unwrap();
        assert_eq!(managed, root.join("bin/pdfium"));
        std::fs::create_dir_all(&managed).unwrap();
        let independent = managed.join(pdfium_lib_filename());
        std::fs::write(&independent, b"not a native library").unwrap();
        assert_eq!(
            independent_candidate(&independent, &root),
            Some(independent)
        );
        assert!(allowed_pdfium_path(
            &root.join("plugins-independent/helper"),
            &root
        ));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pdfium_rejects_plugin_symlinks_in_both_directions() {
        use std::os::unix::fs::symlink;
        let root = test_root();
        let plugin = root.join("plugins/study").join(pdfium_lib_filename());
        let independent = root.join("independent").join(pdfium_lib_filename());
        std::fs::create_dir_all(plugin.parent().unwrap()).unwrap();
        std::fs::create_dir_all(independent.parent().unwrap()).unwrap();
        std::fs::write(&plugin, b"not a native library").unwrap();
        std::fs::write(&independent, b"not a native library").unwrap();

        let disguised_plugin = root.join("independent/disguised-helper");
        symlink(&plugin, &disguised_plugin).unwrap();
        assert!(independent_candidate(&disguised_plugin, &root).is_none());
        let plugin_directory_alias = root.join("plugin-alias");
        symlink(plugin.parent().unwrap(), &plugin_directory_alias).unwrap();
        assert!(
            independent_candidate(&plugin_directory_alias.join(pdfium_lib_filename()), &root)
                .is_none()
        );

        let lexical_plugin = root.join("plugins/study/linked-independent");
        symlink(&independent, &lexical_plugin).unwrap();
        assert!(independent_candidate(&lexical_plugin, &root).is_none());
        let independent_alias = root.join("independent-alias");
        symlink(&independent, &independent_alias).unwrap();
        assert!(independent_candidate(&independent_alias, &root).is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn pdfium_rejects_symlinked_managed_directories_before_files_exist() {
        use std::os::unix::fs::symlink;
        let root = test_root();
        let app_data = root.join("app");
        let external_plugins = root.join("plugin-files");
        std::fs::create_dir_all(&app_data).unwrap();
        std::fs::create_dir_all(&external_plugins).unwrap();
        symlink(&external_plugins, app_data.join("plugins")).unwrap();
        assert!(!allowed_pdfium_path(
            &external_plugins.join("study/missing-library"),
            &app_data
        ));
        symlink(&external_plugins, app_data.join("bin")).unwrap();
        assert!(managed_dir(&app_data).is_none());
        assert!(!allowed_pdfium_path(
            &app_data.join("bin/pdfium/missing-library"),
            &app_data
        ));
        std::fs::remove_dir_all(root).unwrap();
    }
}
