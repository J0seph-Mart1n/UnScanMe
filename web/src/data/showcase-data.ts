export type SampleId = "invoice" | "lab" | "research";
export type WorkbenchMode = "image" | "prompt" | "pdf";
export type OutputTab = "preview" | "visual" | "source";
export type OsTab = "linux" | "macos" | "windows";
export type PagePresetId = "a4-portrait" | "a4-landscape" | "letter-portrait";

export type CapabilityIconName =
  | "pdf"
  | "table"
  | "chat"
  | "crop"
  | "paste"
  | "page";

export interface TableRowItem {
  col1: string;
  col2: string;
  col3: string;
}

export interface SampleDoc {
  id: SampleId;
  title: string;
  fileName: string;
  scanSize: string;
  pdfSize: string;
  pdfPages: number;
  heading: string;
  subtitle: string;
  headers: [string, string, string];
  rows: TableRowItem[];
  totalLabel: string;
  totalValue: string;
  note: string;
}

export interface PagePreset {
  id: PagePresetId;
  label: string;
  cssSize: string;
  marginMm: number;
}

export const WORKBENCH_MODES: { id: WorkbenchMode; label: string; badge: string }[] = [
  { id: "image", label: "Image to PDF", badge: "Vision AI + Preprocess" },
  { id: "prompt", label: "Prompt to PDF", badge: "Text -> HTML5" },
  { id: "pdf", label: "PDF to Editable HTML", badge: "Poppler BBox + Raster" },
];

export const PAGE_PRESETS: Record<PagePresetId, PagePreset> = {
  "a4-portrait": {
    id: "a4-portrait",
    label: "A4 · Portrait · 15mm",
    cssSize: "A4 portrait",
    marginMm: 15,
  },
  "a4-landscape": {
    id: "a4-landscape",
    label: "A4 · Landscape · 10mm",
    cssSize: "A4 landscape",
    marginMm: 10,
  },
  "letter-portrait": {
    id: "letter-portrait",
    label: "Letter · Portrait · 15mm",
    cssSize: "letter portrait",
    marginMm: 15,
  },
};

export const SAMPLES: Record<SampleId, SampleDoc> = {
  invoice: {
    id: "invoice",
    title: "Engineering Invoice",
    fileName: "scan_invoice_2048.jpg",
    scanSize: "3.8 MB JPEG Scan",
    pdfSize: "42 KB Vector PDF",
    pdfPages: 2,
    heading: "INVOICE #INV-2026-089",
    subtitle: "Apex Robotics Lab · Issued: Nov 18, 2026 · Net 15",
    headers: ["Deliverable", "Qty / Hours", "Line Total"],
    rows: [
      { col1: "Embedded Rust Firmware Audit", col2: "24 hrs", col3: "$2,880.00" },
      { col1: "PCB Thermal Imaging & Calibration", col2: "1 unit", col3: "$950.00" },
      { col1: "Local Vision Model Quantization", col2: "8 hrs", col3: "$1,120.00" },
    ],
    totalLabel: "Total Due",
    totalValue: "$4,950.00",
    note: "Payment via ACH / Wire. Verified offline on local workstation.",
  },
  lab: {
    id: "lab",
    title: "Clinical Lab Report",
    fileName: "pathology_panel_scan.png",
    scanSize: "5.1 MB PNG Scan",
    pdfSize: "56 KB Vector PDF",
    pdfPages: 3,
    heading: "COMPREHENSIVE METABOLIC PANEL",
    subtitle: "Patient ID: #8841-A · Specimen: Venous Blood · Local Pipeline",
    headers: ["Analyte", "Measured", "Reference Range"],
    rows: [
      { col1: "Fasting Glucose", col2: "89 mg/dL", col3: "70 – 99 mg/dL" },
      { col1: "Hemoglobin A1c", col2: "5.2 %", col3: "4.0 – 5.6 %" },
      { col1: "TSH (Thyroid Stimulating)", col2: "1.94 mIU/L", col3: "0.45 – 4.50 mIU/L" },
    ],
    totalLabel: "Panel Summary",
    totalValue: "WITHIN NORMAL LIMITS",
    note: "Processed 100% locally on workstation — zero cloud exposure.",
  },
  research: {
    id: "research",
    title: "Benchmark Table",
    fileName: "paper_page4_photo.webp",
    scanSize: "2.9 MB Camera Photo",
    pdfSize: "38 KB Vector PDF",
    pdfPages: 4,
    heading: "Table 3: Local VLM Document Reconstruction",
    subtitle: "Evaluated on 4 GB Laptop GPU (RTX 3050 Mobile · num_ctx = 8192)",
    headers: ["Model / Engine", "Structure Score", "Memory Footprint"],
    rows: [
      { col1: "Qwen2.5-VL 3B (Q4_K_M)", col2: "97.8% ANLS", col3: "2.1 GB VRAM" },
      { col1: "Qwen3-VL 4B (Q4_K_M)", col2: "98.6% ANLS", col3: "2.8 GB VRAM" },
      { col1: "Legacy Tesseract OCR", col2: "74.2% ANLS", col3: "Loses table grid" },
    ],
    totalLabel: "Recommended Default",
    totalValue: "Qwen2.5-VL 3B / Qwen3-VL 4B",
    note: "HTML5 semantic table representation preserves borders and alignment.",
  },
};

export const REFINE_PRESETS = [
  "Highlight the totals row in emerald",
  "Add a late-fee & warranty note row",
  "Switch typography to classic serif",
] as const;

export const CAPABILITIES: {
  id: string;
  tag: string;
  title: string;
  description: string;
  detail: string;
  icon: CapabilityIconName;
}[] = [
  {
    id: "pdf-import",
    tag: "Offline & AI PDF Import",
    title: "PDF-to-Editable-HTML Import",
    description:
      "Ingest existing multi-page PDFs with instant Poppler BBox table clustering (no AI wait) or switch to Vision AI transcription per rasterized page.",
    detail: "pdftotext -bbox-layout · pdftoppm · mutool · pure-Rust stream fallback",
    icon: "pdf",
  },
  {
    id: "table-editor",
    tag: "Canvas & Grid Editing",
    title: "Visual Canvas & Interactive Table Editor",
    description:
      "Edit text inline, drag whole tables via the move handle, nudge elements on a 4px snap grid, insert shapes/images, and add/remove table rows & columns.",
    detail: "Tab / Shift+Tab cell navigation · 50-step Undo/Redo · clean HTML5 export",
    icon: "table",
  },
  {
    id: "refine-ai",
    tag: "Conversational Edits",
    title: "Refine with AI Follow-Up Chat",
    description:
      "Iterate on generated or imported HTML with follow-up prompts while protecting inline base64 images via compact placeholder tokens.",
    detail: "Preserves data:image URIs · multi-turn context · targeted CSS/HTML edits",
    icon: "chat",
  },
  {
    id: "image-prep",
    tag: "Pure-Rust Preprocessing",
    title: "Image Crop, Rotate & Auto-Deskew",
    description:
      "Straighten crooked camera scans automatically with projection-profile deskew detection (-15° to +15°), rotate 90°, or drag-crop regions before inference.",
    detail: "Zero external CV runtime · bilinear resampling · non-destructive Reset",
    icon: "crop",
  },
  {
    id: "clipboard-dnd",
    tag: "Zero-Friction Input",
    title: "Clipboard Paste (Ctrl+V) & Drag-and-Drop",
    description:
      "Paste screenshots directly with Ctrl+V (with wl-paste, xclip, pngpaste, and Windows fallback) or drop images and PDF files anywhere onto the workbench.",
    detail: "PNG/JPG/WebP/TIFF/BMP & %PDF- validation · normalized PNG staging",
    icon: "paste",
  },
  {
    id: "page-setup",
    tag: "Print Geometry Control",
    title: "Toolbar Page Setup & Vector PDF Export",
    description:
      "Configure A4, Letter, Legal, A3, or A5 paper sizes, Portrait/Landscape orientation, and millimeter margins cleanly via idempotent @page CSS rules.",
    detail: "Isolated Chromium / Edge headless profile · 100% searchable vector PDF",
    icon: "page",
  },
];

export const PIPELINE_STEPS = [
  {
    step: "01",
    title: "Ingest & Preprocess",
    body: "Drop an image or PDF, paste with Ctrl+V, or enter a prompt. Pure-Rust image ops crop, rotate, and auto-deskew before staging.",
  },
  {
    step: "02",
    title: "Reconstruct Semantic HTML5",
    body: "Send scans to local Ollama Vision models or extract Poppler BBox tables offline into real <table>, <thead>, <tbody>, and CSS layouts.",
  },
  {
    step: "03",
    title: "Visual Edit & Refine with AI",
    body: "Double-click text, drag whole tables, insert shapes, run row/column actions, or send targeted follow-up edits in the AI chat drawer.",
  },
  {
    step: "04",
    title: "Page Setup & Export PDF",
    body: "Inject @page size and margin rules from the top toolbar and print via an isolated headless Chromium or Edge process.",
  },
];

export const OS_COMMANDS: Record<
  OsTab,
  { label: string; badge: string; browserNote: string; commands: string }
> = {
  linux: {
    label: "Linux (Arch / Omarchy / Ubuntu)",
    badge: "WebKitGTK + Poppler + Chromium",
    browserNote:
      "Auto-detects /usr/bin/chromium, google-chrome-stable, brave, wl-paste/xclip, and poppler-utils.",
    commands: `# 1. Install desktop & PDF utilities + Ollama
sudo pacman -S xdotool poppler wl-clipboard ollama-cuda
sudo systemctl enable --now ollama

# 2. Build and launch from desktop/
cd desktop && dx serve --desktop`,
  },
  macos: {
    label: "macOS (Apple Silicon & Intel)",
    badge: "WKWebView + Chrome / Edge",
    browserNote:
      "Auto-detects Google Chrome.app, Chromium.app, Brave Browser.app, or Microsoft Edge.app.",
    commands: `# 1. Install Ollama & optional Poppler / clipboard tools
brew install --cask ollama
brew install poppler pngpaste
ollama serve &

# 2. Build and launch from desktop/
cd desktop && dx serve --desktop`,
  },
  windows: {
    label: "Windows 10 & 11 (x64 / ARM64)",
    badge: "WebView2 + Built-in MS Edge",
    browserNote:
      "Uses pre-installed Microsoft Edge (msedge.exe) or Chrome with normalized file:///C:/ URLs and native PowerShell clipboard.",
    commands: `# 1. Install Ollama via Winget
winget install Ollama.Ollama

# 2. Build and launch from desktop/
cd desktop; dx serve --desktop`,
  },
};

export const FAQS = [
  {
    q: "Why convert documents into semantic HTML5 before exporting to PDF?",
    a: "Image-wrapped PDFs are heavy and uneditable, and raw OCR text dumps break table alignment. Reconstructing into semantic HTML5 + CSS lets you inspect, visually move tables, edit individual cells, refine styles with AI, and print crisp searchable vector PDFs via Chromium.",
  },
  {
    q: "Can I import existing PDFs without running a vision AI model?",
    a: "Yes. DocForge parses Poppler pdftotext -bbox-layout coordinates (clustering split flows into aligned HTML <table> rows), falls back to -layout or uncompressed PDF text streams offline, or lets you run Vision AI on any rasterized page when a PDF is a pure scan.",
  },
  {
    q: "How does the Visual Editor protect my exported HTML?",
    a: "All selection outlines, 8-point resize handles, and drag pills live inside an isolated #__df_editor_ui__ layer. Before syncing back to Rust or printing to PDF, DocForge clones the DOM and strips all editor chrome and temporary contenteditable attributes.",
  },
  {
    q: "What happens if my Ollama model is not installed yet?",
    a: "The workbench checks localhost:11434 automatically and highlights missing models in a top banner with one-click download and live MB/percentage streaming progress—no terminal required.",
  },
  {
    q: "Does any document image, PDF, or prompt leave my machine?",
    a: "No. Image preprocessing, PDF extraction, visual canvas editing, local Ollama inference, and headless Chromium/Edge PDF rendering all run 100% locally on your hardware.",
  },
];
