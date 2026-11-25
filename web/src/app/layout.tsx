import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "UnScanMe — Unscan Faster, Edit with Confidence",
  description:
    "Turn document photos, crooked scans, and locked PDFs into clean editable documents and searchable vector PDFs — 100% locally on Windows, macOS, and Linux.",
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
