import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "UnScanMe — Unscan Faster, Edit with Confidence",
  description:
    "Turn document photos, crooked scans, and locked PDFs into editable HTML, interactive tables, and searchable vector PDFs — 100% offline with Ollama or faster with your own Gemini, OpenAI, or Claude API keys.",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}
