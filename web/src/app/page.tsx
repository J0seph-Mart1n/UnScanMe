import React from "react";
import { FaqSection } from "../components/FaqSection";
import { FeatureMatrix } from "../components/FeatureMatrix";
import { ShieldIcon } from "../components/Icons";
import { InstallSection } from "../components/InstallSection";
import { Navbar } from "../components/Navbar";
import { WorkbenchDemo } from "../components/WorkbenchDemo";

export default function ShowcasePage() {
  return (
    <div id="top">
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      <Navbar />

      <main id="main-content">
        <section className="hero" aria-labelledby="hero-heading">
          <div className="container">
            <div className="hero-pill">
              <ShieldIcon className="icon-xs" />
              <span>Rust + Dioxus 0.7 Desktop · Ollama Vision AI · 100% Offline</span>
            </div>

            <h1 id="hero-heading">
              Turn Scans, Prompts & PDFs Into{" "}
              <span className="hero-accent">Editable HTML5 & Vector PDFs</span>
            </h1>

            <p className="hero-lead">
              i hate scans pairs local Ollama vision models and Poppler BBox extraction with a visual
              canvas editor, interactive table controls, pure-Rust crop/deskew, and AI follow-up
              refinement — without sending a single byte to the cloud.
            </p>

            <div className="hero-actions">
              <a href="#install" className="btn btn-primary">
                Setup Desktop Workbench
              </a>
              <a href="#workbench" className="btn">
                Explore Interactive Preview
              </a>
            </div>

            <ul className="hero-specs" aria-label="Core capabilities">
              <li>PDF-to-Editable-HTML Import</li>
              <li>Interactive Table & Canvas Editor</li>
              <li>Refine with AI Chat</li>
              <li>Crop, Rotate & Auto-Deskew</li>
              <li>Ctrl+V Paste & Drag-and-Drop</li>
              <li>Toolbar Page Setup</li>
            </ul>
          </div>
        </section>

        <WorkbenchDemo />
        <FeatureMatrix />
        <InstallSection />
        <FaqSection />
      </main>
    </div>
  );
}
