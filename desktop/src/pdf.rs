//! HTML -> PDF conversion using headless Chromium / Chrome / Edge.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Locate an installed Chromium-based browser on Linux, macOS, or Windows.
pub fn detect_chromium() -> String {
    if cfg!(target_os = "windows") {
        let mut roots = Vec::new();
        for var in ["PROGRAMFILES", "PROGRAMFILES(X86)", "LOCALAPPDATA"] {
            if let Ok(val) = std::env::var(var) {
                roots.push(PathBuf::from(val));
            }
        }
        if roots.is_empty() {
            roots.push(PathBuf::from(r"C:\Program Files"));
            roots.push(PathBuf::from(r"C:\Program Files (x86)"));
        }

        let rel_candidates = [
            r"Google\Chrome\Application\chrome.exe",
            r"Chromium\Application\chrome.exe",
            r"BraveSoftware\Brave-Browser\Application\brave.exe",
            r"Microsoft\Edge\Application\msedge.exe",
        ];

        for rel in rel_candidates {
            for root in &roots {
                let candidate = root.join(rel);
                if candidate.is_file() {
                    return candidate.to_string_lossy().into_owned();
                }
            }
        }
        return "msedge".into();
    }

    if cfg!(target_os = "macos") {
        let candidates = [
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
            "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
            "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
        ];
        for c in candidates {
            if Path::new(c).is_file() {
                return c.into();
            }
        }
        return candidates[0].into();
    }

    // Linux / other Unix
    let candidates = [
        "chromium",
        "chromium-browser",
        "google-chrome-stable",
        "google-chrome",
        "brave",
        "microsoft-edge-stable",
        "microsoft-edge",
    ];
    if let Some(path_var) = std::env::var_os("PATH") {
        for name in candidates {
            for dir in std::env::split_paths(&path_var) {
                if dir.join(name).is_file() {
                    return name.into();
                }
            }
        }
    }
    for dir in ["/usr/bin", "/usr/local/bin", "/snap/bin"] {
        for name in candidates {
            let p = Path::new(dir).join(name);
            if p.is_file() {
                return p.to_string_lossy().into_owned();
            }
        }
    }
    "chromium".into()
}

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Convert a local filesystem path into a cross-platform `file://` URL.
fn path_to_file_url(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if cfg!(windows) {
        let cleaned = raw.trim_start_matches(r"\\?\").replace('\\', "/");
        format!("file:///{}", cleaned.trim_start_matches('/'))
    } else {
        format!("file://{raw}")
    }
}

/// Render `html` to a PDF file at `output` using the given Chromium executable.
pub async fn html_to_pdf(chromium: &str, html: &str, output: &Path) -> Result<()> {
    let browser = if chromium.trim().is_empty() {
        detect_chromium()
    } else {
        chromium.trim().to_string()
    };

    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let work_dir = std::env::temp_dir().join(format!("unscanme-{}-{stamp}", std::process::id()));
    let _guard = TempDirGuard(work_dir.clone());
    tokio::fs::create_dir_all(&work_dir).await?;
    let html_path = work_dir.join("document.html");
    tokio::fs::write(&html_path, html).await?;

    let file_url = path_to_file_url(&html_path);

    let out = tokio::time::timeout(
        std::time::Duration::from_secs(90),
        tokio::process::Command::new(&browser)
            .kill_on_drop(true)
            .arg("--headless")
            .arg("--disable-gpu")
            .arg("--no-first-run")
            .arg("--no-default-browser-check")
            .arg("--disable-extensions")
            .arg("--disable-background-networking")
            .arg("--disable-sync")
            .arg("--disable-default-apps")
            .arg("--disable-component-update")
            .arg("--metrics-recording-only")
            .arg("--no-pdf-header-footer")
            // Separate profile so it never collides with a running browser instance.
            .arg(format!("--user-data-dir={}", work_dir.join("profile").display()))
            .arg(format!("--print-to-pdf={}", output.display()))
            .arg(&file_url)
            .output(),
    )
    .await
    .map_err(|_| anyhow::anyhow!("Chromium PDF export timed out after 90s"))?
    .with_context(|| {
        format!("failed to start `{browser}`. Check the Chromium / Chrome path in Settings")
    })?;

    if !out.status.success() || !output.exists() {
        bail!(
            "Chromium failed ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temp_dir_guard_cleans_up_on_drop() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir =
            std::env::temp_dir().join(format!("unscanme_guard_{}_{stamp}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("probe.txt"), b"ok").unwrap();
        assert!(dir.exists());
        {
            let _guard = TempDirGuard(dir.clone());
        }
        assert!(!dir.exists());
    }
}
