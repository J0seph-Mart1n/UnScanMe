---
name: dioxus-desktop-dev
description: >-
  Build, verify, and troubleshoot the Dioxus 0.7 desktop app in img_to_pdf/desktop.
  Use when running dx build/serve, diagnosing Linux linker or asset bundling errors,
  or working on Ollama model pulling and headless Chromium PDF rendering.
---

# Dioxus Desktop Development & Troubleshooting

## Standard Verification Command
Always run build checks from `desktop/`:

```bash
cd /home/joseph/Coding/Rust_Projects/img_to_pdf/desktop && cargo check --offline && dx build --desktop
```

Confirm both exit with code `0`, zero warnings, and `Client build completed successfully!`.

## Known Failure Modes & Fixes

### 1. Linker error: `unable to find library -lxdo` (Linux / Arch / Omarchy)
- **Cause**: `dioxus-desktop` -> `muda` -> `libxdo-sys` links dynamically against `libxdo.so`.
- **Fix**: Install `xdotool` (and optionally `libappindicator-gtk3` if tray icons are used):
  ```bash
  sudo pacman -S xdotool
  ```

### 2. Asset error: `Failed to copy asset .../assets/...: No such file or directory`
- **Cause**: Dioxus's `asset!()` macro bakes compile-time `CARGO_MANIFEST_DIR` paths into incremental build artifacts inside `target/`. Moving or renaming the crate leaves stale absolute paths.
- **Fix**: Clean the crate's cached build and `dx` bundle artifacts, then rebuild:
  ```bash
  cd desktop
  cargo clean -p img_to_pdf
  rm -rf target/x86_64-unknown-linux-gnu/desktop-dev/build/img_to_pdf target/x86_64-unknown-linux-gnu/desktop-dev/incremental target/dx
  dx build --desktop
  ```

### 3. `dx` panic in `workspace.rs` (`Os { code: 2, kind: NotFound }`)
- **Cause**: `dioxus-cli` 0.7.10 calls `std::fs::canonicalize(default)` on Cargo workspace `default-members` relative to CWD.
- **Fix**: Keep `desktop/` as a standalone Cargo project without a root `[workspace]` `Cargo.toml`, and invoke `dx` from inside `desktop/`.

### 4. Ollama Streaming `/api/pull` Handling
- Buffer incomplete newline-delimited JSON chunks across `response.chunk().await` calls before parsing lines.
- Check for mid-stream `{"error": "..."}` payloads (HTTP status may still be `200 OK` on stream errors like unknown model tags).
- Match installed models case-insensitively and account for implicit `:latest` tags when comparing against `/api/tags`.
