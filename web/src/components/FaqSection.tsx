"use client";

import React, { useState } from "react";
import { FAQS, PLATFORM_DOWNLOADS, type OsTab } from "../data/showcase-data";
import { ArrowRightIcon, ChevronDownIcon, LogoMarkIcon } from "./Icons";

export function FaqSection() {
  const [openIndex, setOpenIndex] = useState<number | null>(0);
  const [selectedOs, setSelectedOs] = useState<OsTab>("windows");

  const activeDownload =
    PLATFORM_DOWNLOADS.find((p) => p.id === selectedOs) ?? PLATFORM_DOWNLOADS[0];

  return (
    <>
      <section id="faq" className="faq-section" aria-labelledby="faq-heading">
        <div className="container">
          <div className="section-center-head">
            <p className="eyebrow-muted">Frequently Asked Questions</p>
            <h2 id="faq-heading">Everything you need to know</h2>
            <p>
              Quick answers on offline privacy, supported formats, interactive table editing, and
              desktop installers.
            </p>
            <div className="faq-top-actions">
              <a href="#download" className="btn-white-pill">
                Get Started
              </a>
              <a href="#preview" className="btn-dark-pill">
                Try Live Preview
              </a>
            </div>
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

      <div className="footer-stage">
        <div className="container cta-overlap-wrap">
          <section className="cta-painterly-card" aria-labelledby="cta-heading">
            <h2 id="cta-heading">
              Start turning scans into
              <br />
              clean documents today
            </h2>

            <div className="cta-pill-bar">
              <div className="cta-os-tabs" role="group" aria-label="Choose installer OS">
                {PLATFORM_DOWNLOADS.map((plat) => (
                  <button
                    key={plat.id}
                    type="button"
                    className={`cta-os-tab ${selectedOs === plat.id ? "is-active" : ""}`}
                    onClick={() => setSelectedOs(plat.id)}
                  >
                    {plat.title}
                  </button>
                ))}
              </div>
              <a
                href={activeDownload.href}
                download={activeDownload.fileName}
                className="cta-download-btn"
              >
                <span>Download</span>
                <ArrowRightIcon className="icon-xs" />
              </a>
            </div>
          </section>
        </div>

        <footer className="lime-footer">
          <div className="container">
            <div className="lime-footer-grid">
              <div className="lime-footer-brand">
                <a href="#top" className="lime-brand-link">
                  <LogoMarkIcon className="lime-brand-dot" />
                  <span>UnScanMe</span>
                </a>
              </div>

              <div className="lime-footer-col">
                <h3>PRODUCT</h3>
                <ul>
                  <li><a href="#features">Features</a></li>
                  <li><a href="#preview">Live Preview</a></li>
                  <li><a href="#how-it-works">How it works</a></li>
                </ul>
              </div>

              <div className="lime-footer-col">
                <h3>DOWNLOADS</h3>
                <ul>
                  <li>
                    <a
                      href="/downloads/UnScanMe-Setup-0.1.0-x64.exe"
                      download="UnScanMe-Setup-0.1.0-x64.exe"
                    >
                      Windows (.exe)
                    </a>
                  </li>
                  <li>
                    <a
                      href="/downloads/UnScanMe-0.1.0-universal.dmg"
                      download="UnScanMe-0.1.0-universal.dmg"
                    >
                      macOS (.dmg)
                    </a>
                  </li>
                  <li>
                    <a
                      href="/downloads/UnScanMe-Installer-0.1.0-linux-x86_64"
                      download="UnScanMe-Installer-0.1.0-linux-x86_64"
                    >
                      Linux Installer
                    </a>
                  </li>
                </ul>
              </div>

              <div className="lime-footer-col">
                <h3>RESOURCES</h3>
                <ul>
                  <li><a href="#faq">FAQ</a></li>
                  <li><span>100% Local AI</span></li>
                  <li><span>Vector PDF Engine</span></li>
                </ul>
              </div>
            </div>

            <div className="lime-footer-bottom">
              <span>© 2026 UnScanMe. All rights reserved. Runs 100% locally.</span>
            </div>
          </div>
        </footer>
      </div>
    </>
  );
}
