import React from "react";
import { FaqSection } from "../components/FaqSection";
import { FeatureMatrix } from "../components/FeatureMatrix";
import { ArrowRightIcon, DownloadIcon, ShieldIcon } from "../components/Icons";
import { InstallSection } from "../components/InstallSection";
import { Navbar } from "../components/Navbar";
import { WorkbenchDemo } from "../components/WorkbenchDemo";
import { METRICS } from "../data/showcase-data";

export default function ShowcasePage() {
  return (
    <div id="top">
      <a href="#main-content" className="skip-link">
        Skip to main content
      </a>

      <Navbar />

      <main id="main-content">
        <section className="hero" aria-labelledby="hero-heading">
          <div className="hero-bg-grid" aria-hidden="true" />
          <div className="container hero-content">
            <div className="hero-pill">
              <ShieldIcon className="icon-xs" />
              <span>v0.1.0 · Private Local AI Document Suite</span>
            </div>

            <h1 id="hero-heading">
              Turn Scans &amp; PDFs Into{" "}
              <span className="hero-accent">Clean, Editable Documents</span>
            </h1>

            <p className="hero-lead">
              UnScanMe cleans crooked scans, reconstructs real editable tables, and exports
              lightweight vector PDFs — 100% offline on your own computer.
            </p>

            <div className="hero-actions">
              <a href="#download" className="btn btn-primary">
                <DownloadIcon className="icon-sm" />
                <span>Download for Free</span>
              </a>
              <a href="#preview" className="btn">
                <span>Try Live Preview</span>
                <ArrowRightIcon className="icon-sm" />
              </a>
            </div>

            <p className="hero-trust">
              Runs locally on Windows, macOS &amp; Linux · No account or cloud API key required
            </p>
          </div>
        </section>

        <WorkbenchDemo />

        <section className="metrics-section" aria-label="Key product metrics">
          <div className="container">
            <div className="metrics-strip">
              {METRICS.map((item) => (
                <div key={item.value} className="metric-item">
                  <span className="metric-value">{item.value}</span>
                  <span className="metric-label">{item.label}</span>
                </div>
              ))}
            </div>
          </div>
        </section>

        <FeatureMatrix />
        <InstallSection />
        <FaqSection />
      </main>
    </div>
  );
}
