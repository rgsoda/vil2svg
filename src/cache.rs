//! The on-disk cache holding the QMK physical layout.

use std::path::{Path, PathBuf};

const INFO_URL: &str = "https://keyboards.qmk.fm/v1/keyboards/crkbd/rev1/info.json";

pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| PathBuf::from(".cache"));
    base.join("vil2svg")
}

/// QMK physical layout for crkbd, fetched once then reused offline.
pub fn cached_info_json() -> Result<PathBuf, String> {
    let dir = cache_dir();
    let path = dir.join("crkbd-rev1.info.json");
    if path.exists() {
        return Ok(path);
    }
    std::fs::create_dir_all(&dir).map_err(|e| format!("could not create {}: {e}", dir.display()))?;

    let fetched = fetch(INFO_URL).map_err(|e| {
        format!(
            "could not fetch the crkbd layout ({e}).\n  Save it manually to {} and re-run.",
            path.display()
        )
    })?;
    std::fs::write(&path, fetched).map_err(|e| format!("could not write {}: {e}", path.display()))?;
    Ok(path)
}

fn fetch(url: &str) -> Result<String, String> {
    let body = ureq::get(url)
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_to_string()
        .map_err(|e| e.to_string())?;
    // the QMK API wraps it; we want the bare keyboard object
    let payload: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let info = payload
        .get("keyboards")
        .and_then(|k| k.get("crkbd/rev1"))
        .unwrap_or(&payload);
    serde_json::to_string(info).map_err(|e| e.to_string())
}

/// Where `--open` looks for a viewer: `imv` on Linux, `open` on macOS.
pub fn open_svg(path: &Path) {
    let viewer = if cfg!(target_os = "macos") { "open" } else { "imv" };
    let _ = std::process::Command::new(viewer)
        .arg(path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}
