# UnScanMe (`unscanme`) Repository Guidelines

## Repository Structure
- `desktop/`: Standalone Rust + Dioxus 0.7 desktop application (`desktop/Cargo.toml`, `desktop/Dioxus.toml`).
- `web/`: Next.js showcase website (`web/package.json`).
- **Do not add a root Cargo `[workspace]` with `default-members = ["desktop"]`**: `dioxus-cli` 0.7.10 canonicalizes `default-members` relative to CWD instead of workspace root and panics when `dx` is run inside `desktop/`. Always keep `desktop/` as a standalone Cargo crate and run `cargo` / `dx` commands from `desktop/`.

## Dioxus Desktop State & Lifecycle
- **Persist route state at root**: `Router::<Route>` unmounts route components on navigation. Never store workbench state (uploaded images, prompts, generated HTML, active mode) in route-local `use_signal`. Group signals into a `Copy` context struct (`ConvertState`) provided in `App` via `use_context_provider`.
- **Prevent stuck busy UI on unmount**: Because Dioxus cancels component-scoped async tasks on unmount, add a `use_drop` hook using `.peek()` (never `.read()`) to reset transient `Busy` / `Downloading` status back to `Idle`.
- **Re-sync model status on return**: When checking Ollama model availability in `use_effect`, handle both `Ok(false)` (set `MissingModel`) and `Ok(true)` (clear stale `MissingModel` back to `Idle`).

## Visual Editor (`VisualEditor` & `canvas_editor.js`)
- **Gate `Visual Edit` on generated HTML**: Only render the **Visual Edit** tab when `has_html` (`!html.read().trim().is_empty()`) is true, and reset `tab` to `Tab::Preview` whenever HTML is cleared or a new image is picked.
- **Keep `document::eval` channel alive & prevent iframe reload loops**: In `dioxus-desktop`, `NativeDioxusChannel` is garbage-collected if the evaluated script returns. Keep the JS controller awaiting `channel.recv()` in a loop until `EditorCommand::Shutdown` is sent in `use_drop`. Never bind the editor iframe's `srcdoc` reactively to `html`; track `last_synced_html` and only send `EditorCommand::LoadHtml` when `html` changes externally.
- **Isolate editor chrome from serialized HTML**: Render selection frames, resize handles, and coordinate badges inside `#__df_editor_ui__` (`data-df-editor-ui="1"`), and strip `[data-df-editor-ui='1']`, `contenteditable`, and `data-df-editing` on a DOM clone before syncing HTML back to Rust. Embed inserted local images as base64 `data:` URLs so headless Chromium exports offline.

## Desktop UI & UX Invariants
- **Fit pages in viewport without page scrolling**: Use full-viewport flex/grid layouts (`flex: 1; min-height: 0;`) and two-column splits where appropriate (e.g., Settings left column for controls, right column for full-height prompt editor). Confine scrolling inside textareas or capped chip lists (`.chips-scroll`).
- **Top-aligned status banners**: Place connection status, download progress, save confirmation, and error banners at the **top** of the page directly beneath the page header—never below tall form fields.
- **Custom dark dropdowns (avoid native `<select>`)**: In `dioxus-desktop` on Linux (WebKitGTK), native `<select>` elements render light GTK popup widgets that clash with dark themes. Keep `color-scheme: dark;` on `html, body` and implement toolbar dropdowns (`mode-menu`, `shape-menu`) as custom themed popover menus (trigger button + backdrop + menu options).
- **Clear controls & long paths**: Pair persistent workbench state with explicit Clear actions (per-pane Clear + toolbar Clear all). Apply ellipsis truncation (`.path-label` with `min-width: 0; overflow: hidden; text-overflow: ellipsis;`) and `flex-shrink: 0` on small buttons so long file paths never push controls off-screen.
- **Design tokens & iconography**: Use semantic CSS custom properties (`--bg-*`, `--border-*`, `--text-*`, `--accent`, `--radius-*`) in `desktop/assets/styling/main.css` and inline SVG icons (`view_box`, `stroke_width`, `stroke_linecap`, `stroke_linejoin` in Dioxus RSX) rather than decorative emojis.
- **Accessibility (WCAG 2.2 AA)**: Preserve `:focus-visible` outlines, `@media (prefers-reduced-motion: reduce)`, semantic `<button>` elements for interactive empty-state cards (`.empty-card`), and ARIA roles/attributes (`role="toolbar"`, `role="tablist"`, `role="tab"`, `aria_selected`, `aria_haspopup`, `aria_expanded`, `role="status"` / `role="alert"` with `aria_live="polite"`).

## Cross-Platform PDF & Ollama Rules
- **Browser auto-detection**: Never hardcode `"chromium"` as the sole default. Use `pdf::detect_chromium()` across Linux (`$PATH` + `/usr/bin`), macOS (`/Applications/*.app/...`), and Windows (`%PROGRAMFILES%`, `%LOCALAPPDATA%`, including built-in `msedge.exe`).
- **Cross-platform `file://` URLs**: Normalize Windows drive paths with forward slashes (`file:///C:/...`) via `path_to_file_url` before passing them to headless Chromium/Edge.
