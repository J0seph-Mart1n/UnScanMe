import React from "react";
import Image from "next/image";
import { RocketIcon, StarIcon, ZapIcon } from "./Icons";

export function BentoShowcase() {
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

            <figure className="bento-screenshot bento-screenshot-tall">
              <Image
                src="/images/screenshot-scan-cleanup.png"
                alt="UnScanMe desktop application showing a tilted invoice scan inside the white crop box with auto-deskew and rotation controls"
                width={1080}
                height={1080}
                sizes="(max-width: 640px) 100vw, (max-width: 960px) 440px, 340px"
                className="bento-screenshot-img"
              />
            </figure>
          </article>

          {/* Card 2: Top-Right Wide Dark-Olive Card */}
          <article className="bento-wide-olive">
            <div className="olive-rings" aria-hidden="true" />
            <div className="bento-wide-copy">
              <h3>Interactive Table Editor</h3>
              <p>Click any cell, add rows cleanly, or drag whole tables across the page.</p>
            </div>

            <figure className="bento-screenshot bento-screenshot-wide">
              <Image
                src="/images/screenshot-table-editor.png"
                alt="UnScanMe desktop visual table editor with inline cell editing, row and column controls, and a draggable table selection frame"
                width={1400}
                height={920}
                sizes="(max-width: 640px) 100vw, 440px"
                className="bento-screenshot-img"
              />
            </figure>
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
