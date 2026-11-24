export type SampleId = "invoice" | "lab" | "research";
export type OutputTab = "preview" | "visual" | "source";
export type OsTab = "linux" | "macos" | "windows";
export type PagePresetId = "a4-portrait" | "a4-landscape" | "letter-portrait";
export type BentoSpan = "span-4" | "span-3" | "span-2";

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

export interface MetricItem {
  value: string;
  label: string;
}

export interface BentoFeature {
  id: string;
  tag: string;
  title: string;
  description: string;
  icon: CapabilityIconName;
  span: BentoSpan;
}

export interface WorkflowStep {
  step: string;
  title: string;
  body: string;
}

export const METRICS: MetricItem[] = [
  { value: "100% Local", label: "Zero cloud uploads or API keys" },
  { value: "< 50 KB", label: "Crisp, searchable vector PDFs" },
  { value: "3 Platforms", label: "Native Windows, macOS & Linux" },
  { value: "$0 Forever", label: "Free personal desktop edition" },
];

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
    scanSize: "3.8 MB JPEG Scan",
    pdfSize: "42 KB Vector PDF",
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
    scanSize: "5.1 MB PNG Scan",
    pdfSize: "48 KB Vector PDF",
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
    scanSize: "2.9 MB Camera Photo",
    pdfSize: "38 KB Vector PDF",
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

export const CAPABILITIES: BentoFeature[] = [
  {
    id: "table-editor",
    tag: "Visual Editing",
    title: "Interactive Table & Canvas Editor",
    description:
      "Click any cell to edit text, drag whole tables cleanly across the page, add or remove rows and columns, and undo up to 50 steps.",
    icon: "table",
    span: "span-4",
  },
  {
    id: "image-prep",
    tag: "Image Cleanup",
    title: "Smart Crop & Auto-Deskew",
    description:
      "Drag the white crop frame to isolate regions and straighten crooked phone photos in one click.",
    icon: "crop",
    span: "span-2",
  },
  {
    id: "clipboard-dnd",
    tag: "Fast Capture",
    title: "Instant Paste & Drop",
    description:
      "Paste screenshots straight from your clipboard (Ctrl+V) or drop scans and multi-page PDFs anywhere.",
    icon: "paste",
    span: "span-2",
  },
  {
    id: "pdf-import",
    tag: "Document Recovery",
    title: "PDF-to-Editable-HTML Import",
    description:
      "Unlock trapped PDFs instantly with offline table layout recovery, or run local Vision AI on scanned pages without retyping a word.",
    icon: "pdf",
    span: "span-4",
  },
  {
    id: "refine-ai",
    tag: "Follow-Up Prompts",
    title: "Refine with Local AI",
    description:
      "Ask AI in plain English to reformat tables, highlight totals, or adjust typography while keeping embedded images intact.",
    icon: "chat",
    span: "span-3",
  },
  {
    id: "page-setup",
    tag: "Page Setup",
    title: "Print-Ready Vector PDFs",
    description:
      "Choose A4, Letter, or Legal paper sizes, Portrait or Landscape orientation, and exact margins for crisp vector export.",
    icon: "page",
    span: "span-3",
  },
];

export const WORKFLOW_STEPS: WorkflowStep[] = [
  {
    step: "01",
    title: "Drop or paste any scan",
    body: "Drag in a photo, scanned image, or existing PDF — or press Ctrl+V to paste a screenshot directly.",
  },
  {
    step: "02",
    title: "Edit text, tables & layout",
    body: "Clean up crooked scans, edit cells inline, rearrange tables visually, or refine formatting with AI.",
  },
  {
    step: "03",
    title: "Export a crisp vector PDF",
    body: "Pick your paper size and margins, then export a lightweight, 100% searchable PDF in seconds.",
  },
];

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
    q: "Does UnScanMe send my documents or scans to the cloud?",
    a: "Never. Every step — image crop and deskew, PDF text extraction, table editing, local Ollama vision processing, and vector PDF generation — runs 100% offline on your own computer.",
  },
  {
    q: "What file formats can I import and export?",
    a: "You can paste screenshots from your clipboard or drop PNG, JPEG, WebP, TIFF, BMP, and PDF files. UnScanMe exports clean HTML5 and lightweight, searchable vector PDFs.",
  },
  {
    q: "Can I edit tables and fix wrong text after scanning?",
    a: "Yes. Switch to the Table Editor to click and edit any cell, drag whole tables around the page, add or delete rows and columns, or type a follow-up request to refine the layout with AI.",
  },
  {
    q: "Do I need a complex terminal setup to install AI models?",
    a: "No. UnScanMe checks for local Ollama automatically and lets you download recommended lightweight vision models right inside the app with a live progress bar.",
  },
  {
    q: "Which operating systems are supported?",
    a: "UnScanMe includes native desktop installers for Windows 10/11 (64-bit .exe), macOS (Universal .dmg for Apple Silicon & Intel), and all major Linux distributions (Ubuntu, Debian, Mint, Fedora, and Arch).",
  },
];
