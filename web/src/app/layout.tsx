import type { Metadata } from "next";
import "./globals.css";

export const metadata: Metadata = {
  title: "DocForge — Local Document Workbench (Image & PDF to Editable HTML5 & Vector PDF)",
  description:
    "Convert document images, prompts, and PDFs into editable HTML5 with a Visual Table Editor, pure-Rust crop/deskew, AI follow-up refinement, and headless Chromium vector PDF export — 100% locally.",
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
