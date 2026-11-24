# UnScanMe (`unscanme`)

Desktop app (Dioxus 0.7) that converts a photo or scan of a document into a PDF:

1. **Image → HTML**: a vision model running locally in [Ollama](https://ollama.com) transcribes the page into HTML.
2. **Review**: preview the HTML, or edit the source.
3. **HTML → PDF**: headless Chromium prints the HTML to PDF.

Everything runs on your machine. No cloud APIs are used.

## Setup (Arch / Omarchy)

```bash
sudo pacman -S xdotool ollama-cuda      # xdotool is needed to link dioxus-desktop
sudo systemctl enable --now ollama
ollama pull qwen2.5vl:3b                # default model (~3.2 GB, fits a 4 GB GPU)
# alternatives: ollama pull qwen3-vl:4b
```

Chromium (`/usr/bin/chromium`) is used for PDF rendering. You can change its path in **Settings**.

## Run

```bash
dx serve --desktop
```

## Usage

1. **Open image** → **Convert to HTML** (it can take a minute or more on a laptop GPU).
2. Check the **Preview** and fix any mistakes in **HTML source**.
3. **Export PDF** (or **Save HTML**).

Settings are saved to `~/.config/unscanme/settings.json`: Ollama URL, model, prompt, `num_ctx`, temperature, max image size, and Chromium path.

## Code layout

| File | Purpose |
|---|---|
| `src/main.rs` | Window setup, routes, settings context |
| `src/settings.rs` | `Settings` struct, default prompt, load/save |
| `src/ollama.rs` | Image preprocessing, `/api/chat` call, HTML cleanup, model listing |
| `src/pdf.rs` | Headless Chromium `--print-to-pdf` |
| `src/views/convert.rs` | Main converter UI |
| `src/views/settings_page.rs` | Settings UI |

## Tips

- Out of VRAM or too slow? Lower **Max image size** (e.g. 1280) or **num_ctx**.
- Long or dense pages getting cut off? Raise **num_ctx** (e.g. 12288).
- Low temperature (0–0.2) keeps the transcription faithful.
