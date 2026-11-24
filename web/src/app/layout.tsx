import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "UnScanMe — Turn Scans & PDFs Into Clean, Editable Documents",
  description:
    "Convert document photos, scans, and trapped PDFs into editable tables and crisp searchable vector PDFs — 100% locally on Windows, macOS, and Linux.",
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
