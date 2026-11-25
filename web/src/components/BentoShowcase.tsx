"use client";

import React, { useState } from "react";
import { CheckIcon, RocketIcon, StarIcon, ZapIcon } from "./Icons";

type EditorMode = "Cells" | "Rows" | "Export";

export function BentoShowcase() {
  const [mode, setMode] = useState<EditorMode>("Cells");

  return (
    <section id="features" className="bento-section" aria-labelledby="features-heading">
      <div className="container">
        <h2 id="features-heading" className="sr-only">
          Core UnScanMe Features
        </h2>

        <div className="template-bento">
          {/* Card 1: Tall Left Lime Card */}
          <article className="bento-tall-lime">
            <div className="bento-tall-copy">
              <h3>Guided Cleanup For Every Scan</h3>
              <p>
                Straighten crooked phone photos with pure-Rust auto-deskew and crop regions in one
                step.
              </p>
            </div>

            <div className="phone-mockup phone-mockup-bottom">
              <div className="phone-notch" aria-hidden="true" />
              <p className="phone-eyebrow">UnScanMe Studio</p>
              <h4 className="phone-heading">
                Your document
                <br />
                is ready!
              </h4>
              <p className="phone-sub">Clean HTML5 &amp; vector PDF processed offline.</p>

              <div className="phone-lime-card">
                <div className="phone-lime-top">
                  <span>Invoice #089</span>
                  <span className="phone-check-circle" aria-hidden="true">
                    <CheckIcon className="icon-xs" />
                  </span>
                </div>
                <p className="phone-lime-meta">VECTOR PDF · DESKEWED</p>
              </div>
            </div>
          </article>

          {/* Card 2: Top-Right Wide Dark-Olive Card */}
          <article className="bento-wide-olive">
            <div className="olive-rings" aria-hidden="true" />
            <div className="bento-wide-copy">
              <h3>Interactive Table Editor</h3>
              <p>Click any cell, add rows cleanly, or drag whole tables across the page.</p>
            </div>

            <div className="bento-wide-mockup">
              <div className="phone-mockup phone-mockup-side">
                <div className="phone-notch" aria-hidden="true" />
                <div className="phone-search-pill">Search cells or refine...</div>
                <p className="phone-metric-label">Detected tables</p>
                <p className="phone-metric-big">100% editable</p>

                <div className="phone-mode-pills" role="group" aria-label="Editor focus">
                  {(["Cells", "Rows", "Export"] as EditorMode[]).map((tab) => (
                    <button
                      key={tab}
                      type="button"
                      className={`phone-mode-pill ${mode === tab ? "is-active" : ""}`}
                      onClick={() => setMode(tab)}
                    >
                      {tab}
                    </button>
                  ))}
                </div>
              </div>

              <div className="floating-status-pill">
                <span className="floating-status-label">
                  {mode === "Cells"
                    ? "Inline Edit"
                    : mode === "Rows"
                      ? "Table Move"
                      : "Vector Output"}
                </span>
                <div className="floating-status-row">
                  <strong>
                    {mode === "Cells"
                      ? "Tab Navigation"
                      : mode === "Rows"
                        ? "Snap Grid Ready"
                        : "Searchable PDF"}
                  </strong>
                  <span className="floating-status-badge">
                    <CheckIcon className="icon-xs" />
                    <span>Ready</span>
                  </span>
                </div>
              </div>
            </div>
          </article>

          {/* Card 3: Bottom-Middle Dark-Olive Card */}
          <article className="bento-square-olive">
            <p className="square-eyebrow">100% Offline By</p>
            <h3>Zero Cloud</h3>
            <div className="avatar-stack" aria-hidden="true">
              <span className="stack-badge">Win</span>
              <span className="stack-badge">Mac</span>
              <span className="stack-badge">Lin</span>
              <span className="stack-badge">AI</span>
              <span className="stack-badge is-lime">0%</span>
            </div>
            <p className="square-rating">
              <StarIcon className="icon-xs lime-star" />
              <span>Runs purely on local hardware</span>
            </p>
          </article>

          {/* Card 4: Bottom-Right Lime Card */}
          <article className="bento-square-lime">
            <div>
              <h3>Built for Speed</h3>
              <p className="square-lime-sub">
                Lightweight native Rust &amp; Chromium vector PDF output.
              </p>
            </div>

            <div className="speed-metrics">
              <div className="speed-pill">
                <span className="speed-pill-left">
                  <RocketIcon className="icon-xs lime-star" />
                  <span>Vector PDF Size</span>
                </span>
                <span className="speed-pill-val">~45 KB</span>
              </div>
              <div className="speed-pill">
                <span className="speed-pill-left">
                  <ZapIcon className="icon-xs lime-star" />
                  <span>Offline AI Latency</span>
                </span>
                <span className="speed-pill-val">0ms cloud</span>
              </div>
            </div>
          </article>
        </div>
      </div>
    </section>
  );
}
