# i hate scans — Showcase Website (Next.js)

Showcase and download website for the **i hate scans** (`i-hate-scans`) local AI desktop application (`../desktop`).

## Features Included

- **Interactive 3-Step Pipeline Workbench (`#demo`)**:
  - Switch between realistic document scans (*Engineering Invoice*, *Clinical Lab Report*, *Academic Benchmark Table*).
  - Test the **Custom Instructions** prompt box and style toggles (*Sans-Serif vs Serif*, *Highlight Totals*).
  - Preview the **In-App Ollama Model Downloader** progress bar.
  - Inspect all three stages: **Source Image**, **Live HTML Preview**, **Editable HTML5 Source**, and **Print-Ready Vector PDF**.
- **Feature Highlights (`#features`)**: Explains the local Ollama vision architecture, in-app model pulling, custom prompt box, HTML editor, and cross-platform Chromium/Edge PDF engine.
- **Cross-Platform Downloads & Setup (`#downloads`)**: Interactive tabs for **Linux**, **macOS**, and **Windows** with copyable setup commands.
- **Pricing / Edition Comparison (`#pricing`)**: *Community Local ($0)*, *Pro Workstation*, and *Team Self-Hosted*.
- **Interactive FAQ (`#faq`)**: Accordion answering common architecture and hardware questions.

## Getting Started

```bash
cd /home/joseph/Coding/Rust_Projects/img_to_pdf/web
npm install
npm run dev
```

Then open `http://localhost:3000` in your browser.
