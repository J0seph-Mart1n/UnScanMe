//! Clipboard image reading & validation helpers (WebView paste + OS fallback).

use anyhow::{bail, Context, Result};
use base64::Engine;
use image::ImageFormat;
use std::io::Cursor;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::time::{SystemTime, UNIX_EPOCH};

/// Validate raw image bytes via `image::load_from_memory`, normalize to PNG,
/// and return `(png_bytes, data_url)`.
pub fn normalize_clipboard_image(bytes: &[u8]) -> Result<(Vec<u8>, String)> {
    if bytes.is_empty() {
        bail!("clipboard image is empty");
    }
    let img = image::load_from_memory(bytes).context("clipboard data is not a valid image")?;
    let mut out = Cursor::new(Vec::new());
    img.write_to(&mut out, ImageFormat::Png)?;
    let png = out.into_inner();
    let data_url = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&png)
    );
    Ok((png, data_url))
}

/// Decode a `data:image/...;base64,...` string (or raw base64) into normalized PNG bytes + data URL.
pub fn decode_base64_image(raw: &str) -> Result<(Vec<u8>, String)> {
    let b64 = match raw.find("base64,") {
        Some(idx) => &raw[idx + "base64,".len()..],
        None => raw.trim(),
    };
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .context("invalid base64 image payload from clipboard")?;
    normalize_clipboard_image(&decoded)
}

/// Attempt to read an image directly from the OS clipboard (useful on Linux/Wayland/X11,
/// macOS, and Windows when WebView `clipboardData` does not expose bitmap items).
pub async fn read_os_clipboard_image() -> Result<Vec<u8>> {
    #[cfg(target_os = "linux")]
    {
        // Try Wayland `wl-paste` first
        for mime in ["image/png", "image/jpeg", "image/webp", "image/bmp"] {
            if let Ok(out) = tokio::process::Command::new("wl-paste")
                .args(["--no-newline", "--type", mime])
                .output()
                .await
            {
                if out.status.success()
                    && !out.stdout.is_empty()
                    && image::load_from_memory(&out.stdout).is_ok()
                {
                    return Ok(out.stdout);
                }
            }
        }

        // Fall back to X11 `xclip`
        for mime in ["image/png", "image/jpeg"] {
            if let Ok(out) = tokio::process::Command::new("xclip")
                .args(["-selection", "clipboard", "-t", mime, "-o"])
                .output()
                .await
            {
                if out.status.success()
                    && !out.stdout.is_empty()
                    && image::load_from_memory(&out.stdout).is_ok()
                {
                    return Ok(out.stdout);
                }
            }
        }

        bail!("No image found in clipboard (or install `wl-clipboard` / `xclip`).")
    }

    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = tokio::process::Command::new("pngpaste").arg("-").output().await {
            if out.status.success() && !out.stdout.is_empty() {
                if image::load_from_memory(&out.stdout).is_ok() {
                    return Ok(out.stdout);
                }
            }
        }

        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let tmp = std::env::temp_dir().join(format!("df_clip_{stamp}.png"));
        let script = format!(
            "set theFile to POSIX file \"{}\"\n\
             try\n\
               set imgData to the clipboard as «class PNGf»\n\
               set fRef to open for access theFile with write permission\n\
               write imgData to fRef\n\
               close access fRef\n\
             on error\n\
               error \"no_image\"\n\
             end try",
            tmp.display()
        );
        let res = tokio::process::Command::new("osascript")
            .args(["-e", &script])
            .output()
            .await;
        let bytes = tokio::fs::read(&tmp).await;
        let _ = tokio::fs::remove_file(&tmp).await;
        if let (Ok(out), Ok(data)) = (res, bytes) {
            if out.status.success() && !data.is_empty() {
                return Ok(data);
            }
        }
        bail!("No image found in clipboard.")
    }

    #[cfg(target_os = "windows")]
    {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
        let tmp = std::env::temp_dir().join(format!("df_clip_{stamp}.png"));
        let ps = format!(
            "Add-Type -AssemblyName System.Windows.Forms; \
             $img = [System.Windows.Forms.Clipboard]::GetImage(); \
             if ($img -ne $null) {{ $img.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); exit 0 }} else {{ exit 1 }}",
            tmp.to_string_lossy().replace('\'', "''")
        );
        let res = tokio::process::Command::new("powershell")
            .args(["-NoProfile", "-STA", "-Command", &ps])
            .output()
            .await;
        let bytes = tokio::fs::read(&tmp).await;
        let _ = tokio::fs::remove_file(&tmp).await;
        if let (Ok(out), Ok(data)) = (res, bytes) {
            if out.status.success() && !data.is_empty() {
                return Ok(data);
            }
        }
        bail!("No image found in clipboard.")
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = SystemTime::now().duration_since(UNIX_EPOCH);
        bail!("Clipboard image reading is not supported on this platform.")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{DynamicImage, Rgba, RgbaImage};

    #[test]
    fn normalize_and_decode_data_url_round_trip() {
        let img = RgbaImage::from_pixel(4, 4, Rgba([34, 197, 94, 255]));
        let mut buf = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(img).write_to(&mut buf, ImageFormat::Png).unwrap();

        let (png, url) = normalize_clipboard_image(&buf.into_inner()).unwrap();
        assert!(url.starts_with("data:image/png;base64,"));
        assert!(!png.is_empty());

        let (png2, url2) = decode_base64_image(&url).unwrap();
        assert_eq!(png, png2);
        assert_eq!(url, url2);
    }

    #[test]
    fn rejects_invalid_or_empty_bytes() {
        assert!(normalize_clipboard_image(&[]).is_err());
        assert!(normalize_clipboard_image(b"not an image").is_err());
        assert!(decode_base64_image("data:image/png;base64,!!!invalid!!!").is_err());
    }
}
