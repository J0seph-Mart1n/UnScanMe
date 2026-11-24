"use client";

import React, { useState } from "react";
import { FAQS } from "../data/showcase-data";
import { ChevronDownIcon } from "./Icons";

export function FaqSection() {
  const [openIndex, setOpenIndex] = useState<number | null>(0);

  return (
    <>
      <section id="faq" className="section-block" aria-labelledby="faq-heading">
        <div className="container">
          <div className="section-header">
            <p className="section-tag">Questions & Technical Notes</p>
            <h2 id="faq-heading">Frequently Asked Questions</h2>
            <p>
              Architecture details covering local inference, PDF extraction, visual canvas editing,
              and headless printing.
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

      <footer className="footer">
        <div className="container footer-inner">
          <div>
            <strong>i hate scans</strong> — Rust + Dioxus 0.7 Desktop Document Workbench
          </div>
          <div className="footer-meta">
            <span>Ollama Vision AI</span>
            <span aria-hidden="true">·</span>
            <span>Poppler PDF Import</span>
            <span aria-hidden="true">·</span>
            <span>Headless Chromium / Edge PDF</span>
          </div>
        </div>
      </footer>
    </>
  );
}
