export type SampleId = "invoice" | "lab" | "research";
export type OutputTab = "preview" | "visual" | "source";
export type OsTab = "windows" | "macos" | "linux";
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
  shortLabel: string;
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

export interface TestimonialItem {
  id: string;
  name: string;
  initials: string;
  role: string;
  org: string;
  category: string;
  quote: string;
}

export interface WorkflowStep {
  step: string;
  title: string;
  body: string;
  icon: "paste" | "table" | "rocket";
}

export interface PlatformDownload {
  id: OsTab;
  title: string;
  subtitle: string;
  buttonLabel: string;
  href: string;
  fileName: string;
  steps: [string, string];
  commandHint?: string;
}

export const MARQUEE_ITEMS: string[] = [
  "Ollama ↔ Cloud API Switch",
  "Gemini · OpenAI · Claude",
  "Editable PDF-to-HTML",
  "Parallel Offline OCR",
  "Precision Visual Editor",
  "Live Stream & Instant Stop",
  "Pure-Rust Auto-Deskew",
  "Searchable Vector PDFs",
];

export const STATEMENT_TEXT =
  "Use UnScanMe to unlock scans and PDFs switching in one click between 100% offline Ollama and fast cloud API keys, with live conversion progress, instant cancellation, and a visual editor.";

export const PAGE_PRESETS: Record<PagePresetId, PagePreset> = {
  "a4-portrait": {
    id: "a4-portrait",
    label: "A4 Portrait · 15mm",
    cssSize: "A4 portrait",
    marginMm: 15,
  },
  "a4-landscape": {
    id: "a4-landscape",
    label: "A4 Landscape · 10mm",
    cssSize: "A4 landscape",
    marginMm: 10,
  },
  "letter-portrait": {
    id: "letter-portrait",
    label: "Letter Portrait · 15mm",
    cssSize: "letter portrait",
    marginMm: 15,
  },
};

export const SAMPLES: Record<SampleId, SampleDoc> = {
  invoice: {
    id: "invoice",
    shortLabel: "Invoice",
    title: "Engineering Invoice",
    fileName: "scan_invoice_2048.jpg",
    scanSize: "3.8 MB Scan",
    pdfSize: "42 KB PDF",
    pdfPages: 2,
    heading: "INVOICE #INV-2026-089",
    subtitle: "Apex Robotics Lab · Issued Nov 18, 2026 · Net 15",
    headers: ["Deliverable", "Qty / Hours", "Line Total"],
    rows: [
      { col1: "Embedded Rust Firmware Audit", col2: "24 hrs", col3: "$2,880.00" },
      { col1: "PCB Thermal Calibration", col2: "1 unit", col3: "$950.00" },
      { col1: "Local Vision Model Tuning", col2: "8 hrs", col3: "$1,120.00" },
    ],
    totalLabel: "Total Due",
    totalValue: "$4,950.00",
    note: "Payment via ACH / Wire. Processed offline on local workstation.",
  },
  lab: {
    id: "lab",
    shortLabel: "Lab Report",
    title: "Clinical Lab Report",
    fileName: "pathology_panel_scan.png",
    scanSize: "5.1 MB Scan",
    pdfSize: "48 KB PDF",
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
    shortLabel: "Benchmark",
    title: "Benchmark Table",
    fileName: "paper_page4_photo.webp",
    scanSize: "2.9 MB Photo",
    pdfSize: "38 KB PDF",
    pdfPages: 4,
    heading: "Table 3: Local Document Reconstruction",
    subtitle: "Evaluated on 4 GB Laptop GPU · Fully Offline",
    headers: ["Engine", "Table Accuracy", "Memory Footprint"],
    rows: [
      { col1: "Qwen2.5-VL 3B (Local)", col2: "97.8% ANLS", col3: "2.1 GB VRAM" },
      { col1: "Qwen3-VL 4B (Local)", col2: "98.6% ANLS", col3: "2.8 GB VRAM" },
      { col1: "Legacy OCR Text Dump", col2: "74.2% ANLS", col3: "Loses table grid" },
    ],
    totalLabel: "Recommended Default",
    totalValue: "Qwen2.5-VL 3B / Qwen3-VL 4B",
    note: "Clean semantic HTML tables preserve alignment and crisp typography.",
  },
};

export const REFINE_PRESETS = [
  "Highlight totals",
  "Add warranty row",
  "Switch to serif",
] as const;

export const TESTIMONIALS: TestimonialItem[] = [
  {
    id: "engineering",
    name: "Michael Torres",
    initials: "MT",
    role: "Head of Operations",
    org: "Apex Robotics Lab",
    category: "Engineering Labs",
    quote:
      "We used to retype vendor spec sheets and calibration tables by hand every week. UnScanMe turns crooked phone photos into editable tables and sub-50 KB vector PDFs completely offline.",
  },
  {
    id: "clinical",
    name: "Amanda Chen",
    initials: "AC",
    role: "Clinical Informatics Lead",
    org: "Harborview Pathology",
    category: "Clinical Teams",
    quote:
      "Patient forms cannot touch external APIs. Running local Ollama vision plus instant Poppler table extraction right on our workstations gives us clean digital records with zero cloud exposure.",
  },
  {
    id: "legal",
    name: "David Patterson",
    initials: "DP",
    role: "Managing Partner",
    org: "Northbridge Counsel",
    category: "Legal & Finance",
    quote:
      "Being able to crop a scanned exhibit with the white resize box, adjust broken columns in the visual editor, and print a searchable PDF saved our paralegals hours on every closing.",
  },
  {
    id: "research",
    name: "Elena Rostova",
    initials: "ER",
    role: "ML Systems Researcher",
    org: "OpenBench Institute",
    category: "Academic Research",
    quote:
      "Other tools dump scrambled plain text. UnScanMe reconstructs real semantic HTML tables we can drag, tweak in a sentence with AI, and archive cleanly for our lab notebooks.",
  },
];

export const WORKFLOW_STEPS: WorkflowStep[] = [
  {
    step: "01",
    title: "Drop, paste, or pick your AI mode",
    body: "Drag in a camera photo, crooked scan, or locked multi-page PDF — or press Ctrl+V — and flip between offline Ollama or Gemini, OpenAI, and Claude API keys.",
    icon: "paste",
  },
  {
    step: "02",
    title: "Convert, edit & refine visually",
    body: "Stream editable HTML with live progress and a Stop button; crop/auto-deskew, hover-select exact blocks or table cells, drag tables, or refine via AI chat.",
    icon: "table",
  },
  {
    step: "03",
    title: "Export a crisp vector PDF",
    body: "Set your A4 or Letter margins from the top toolbar and export a lightweight, 100% searchable vector PDF via headless Chromium in seconds.",
    icon: "rocket",
  },
];

export const PLATFORM_DOWNLOADS: PlatformDownload[] = [
  {
    id: "windows",
    title: "Windows",
    subtitle: "Windows 10 & 11 (64-bit)",
    buttonLabel: "Download for Windows (.exe)",
    href: "/downloads/UnScanMe-Setup-0.1.0-x64.exe",
    fileName: "UnScanMe-Setup-0.1.0-x64.exe",
    steps: [
      "Double-click UnScanMe-Setup-0.1.0-x64.exe.",
      "Click OK to add Start Menu & Desktop shortcuts.",
    ],
  },
  {
    id: "macos",
    title: "macOS",
    subtitle: "Apple Silicon & Intel",
    buttonLabel: "Download for macOS (.dmg)",
    href: "/downloads/UnScanMe-0.1.0-universal.dmg",
    fileName: "UnScanMe-0.1.0-universal.dmg",
    steps: [
      "Open UnScanMe-0.1.0-universal.dmg.",
      "Open UnScanMe.app to install straight to Applications.",
    ],
  },
  {
    id: "linux",
    title: "Linux",
    subtitle: "Ubuntu, Debian, Mint, Fedora & Arch",
    buttonLabel: "Download Linux Installer",
    href: "/downloads/UnScanMe-Installer-0.1.0-linux-x86_64",
    fileName: "UnScanMe-Installer-0.1.0-linux-x86_64",
    steps: [
      "Allow executing in Properties (or run the command below).",
      "Installer adds your app menu entry and launches UnScanMe.",
    ],
    commandHint:
      "chmod +x ~/Downloads/UnScanMe-Installer-0.1.0-linux-x86_64 && ~/Downloads/UnScanMe-Installer-0.1.0-linux-x86_64",
  },
];

export const FAQS = [
  {
    q: "Can I run 100% offline or use my own cloud API keys?",
    a: "Yes — switch in one click from the top bar. By default every step runs 100% locally via Ollama, Poppler, Tesseract, and Chromium. When you want faster cloud throughput, plug in a Google Gemini, OpenAI, Anthropic Claude, OpenRouter, Groq, or Together API key (stored locally with 0600 permissions or read from environment variables).",
  },
  {
    q: "How does PDF-to-HTML handle scanned vs. digital PDFs?",
    a: "Scanned pages are never wrapped as flat uneditable images. Choose Convert Page (AI) for full layout, styling, and editable HTML elements, or use Fast OCR (Text) / Quick Extract HTML for instant offline parallel Tesseract and Poppler extraction.",
  },
  {
    q: "Can I edit elements, fix tables, and stop a slow generation?",
    a: "Yes. Visual Edit highlights selectable elements on hover so you can click paragraphs, headings, or table cells precisely, edit text inline, or drag entire tables. Every conversion streams live progress with a Stop button to cancel immediately.",
  },
  {
    q: "What file formats can I import and export?",
    a: "Paste screenshots directly from your clipboard or drop PNG, JPEG, WebP, TIFF, BMP, and PDF files. UnScanMe exports clean semantic HTML5 and lightweight, searchable vector PDFs.",
  },
  {
    q: "Which operating systems are supported?",
    a: "UnScanMe provides native desktop installers for Windows 10/11 (64-bit .exe), macOS (Universal .dmg for Apple Silicon & Intel), and all major Linux distributions (Ubuntu, Debian, Mint, Fedora, and Arch).",
  },
];
