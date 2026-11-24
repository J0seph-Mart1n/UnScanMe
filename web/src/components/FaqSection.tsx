"use client";

import React, { useState } from "react";
import { FAQS } from "../data/showcase-data";
import { ArrowRightIcon, ChevronDownIcon, LogoMarkIcon } from "./Icons";

export function FaqSection() {
  const [openIndex, setOpenIndex] = useState<number | null>(0);

  return (
    <>
      <section id="faq" className="section-block" aria-labelledby="faq-heading">
        <div className="container">
          <div className="section-header">
            <p className="section-tag">FAQ</p>
            <h2 id="faq-heading">Frequently asked questions</h2>
            <p>
              Quick answers about privacy, supported file types, local AI models, and downloads.
            </p>
          </div>

          <div className="faq-list">
            {FAQS.map((item, idx) => {
              const isOpen = openIndex === idx;
              const panelId = `faq-panel-${idx}`;
              const btnId = `faq-btn-${idx}`;

              return (
                <div key={item.q} className="faq-item">
                  <h3>
                    <button
                      id={btnId}
                      type="button"
                      className="faq-q"
                      aria-expanded={isOpen}
                      aria-controls={panelId}
                      onClick={() => setOpenIndex(isOpen ? null : idx)}
                    >
                      <span>{item.q}</span>
                      <ChevronDownIcon className={`faq-chevron ${isOpen ? "is-open" : ""}`} />
                    </button>
                  </h3>
                  <div
                    id={panelId}
                    role="region"
                    aria-labelledby={btnId}
                    hidden={!isOpen}
                    className="faq-a"
                  >
                    {item.a}
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </section>

      <section className="section-block" aria-labelledby="cta-heading">
        <div className="container">
          <div className="cta-banner">
            <h2 id="cta-heading">Stop retyping scans today.</h2>
            <p>
              Download UnScanMe for Windows, macOS, or Linux and convert your first scan in seconds.
            </p>
            <div className="hero-actions">
              <a href="#download" className="btn btn-primary">
                <span>Download UnScanMe Free</span>
                <ArrowRightIcon className="icon-sm" />
              </a>
              <a href="#preview" className="btn">
                Try Interactive Preview
              </a>
            </div>
          </div>
        </div>
      </section>

      <footer className="footer">
        <div className="container footer-grid">
          <div className="footer-brand">
            <a href="#top" className="brand">
              <span className="brand-badge">
                <LogoMarkIcon className="icon-sm" />
              </span>
              <span className="brand-name">UnScanMe</span>
            </a>
            <p>
              Turn document scans and PDFs into clean, editable documents and searchable vector PDFs
              — 100% offline.
            </p>
            <span className="status-chip">
              <span className="status-dot" aria-hidden="true" />
              <span>100% Local · Zero Cloud Uploads</span>
            </span>
          </div>

          <div className="footer-col">
            <h3>Product</h3>
            <ul>
              <li><a href="#features">Features</a></li>
              <li><a href="#how-it-works">How it works</a></li>
              <li><a href="#faq">FAQ</a></li>
            </ul>
          </div>

          <div className="footer-col">
            <h3>Downloads</h3>
            <ul>
              <li><a href="/downloads/UnScanMe-Setup-0.1.0-x64.exe" download="UnScanMe-Setup-0.1.0-x64.exe">Windows (.exe)</a></li>
              <li><a href="/downloads/UnScanMe-0.1.0-universal.dmg" download="UnScanMe-0.1.0-universal.dmg">macOS (.dmg)</a></li>
              <li><a href="/downloads/UnScanMe-Installer-0.1.0-linux-x86_64" download="UnScanMe-Installer-0.1.0-linux-x86_64">Linux Installer</a></li>
            </ul>
          </div>

          <div className="footer-col">
            <h3>Stack</h3>
            <ul>
              <li><span>Rust + Dioxus 0.7</span></li>
              <li><span>Local Ollama Vision</span></li>
              <li><span>Vector PDF Engine</span></li>
            </ul>
          </div>
        </div>
      </footer>
    </>
  );
}
