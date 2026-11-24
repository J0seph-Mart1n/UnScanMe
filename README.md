# UnScanMe (`unscanme`)

A cross-platform document conversion suite that turns scanned documents and photos into editable **HTML5** and print-ready **vector PDFs** using 100% local AI models.

## Repository Structure

| Folder | Description |
|---|---|
| [`desktop/`](./desktop) | **Rust + Dioxus 0.7 Desktop Application** — converts document images to HTML via a local Ollama vision model, provides a live HTML preview/editor & custom prompt box, includes an in-app Ollama model downloader, and exports to PDF using headless Chromium / Chrome / Edge. |
| [`web/`](./web) | **Next.js SaaS Showcase Website** — interactive product landing page featuring an Image → HTML → PDF pipeline workbench demo, feature overview, cross-platform installation tabs, editions comparison, and FAQ. |

---

## 1. Run the Desktop Application (`desktop/`)

```bash
cd desktop
dx serve --desktop
```

See [`desktop/README.md`](./desktop/README.md) for full setup and usage instructions on Linux, macOS, and Windows.

---

## 2. Run the Showcase Website (`web/`)

```bash
cd web
npm install
npm run dev
```

Open `http://localhost:3000` in your browser. See [`web/README.md`](./web/README.md) for details.
