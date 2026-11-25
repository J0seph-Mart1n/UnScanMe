"use client";

import React, { useState } from "react";
import { PLATFORM_DOWNLOADS } from "../data/showcase-data";
import { ArrowDownRightIcon, CheckIcon, CopyIcon, MonitorIcon } from "./Icons";

export function InstallSection() {
  const [copiedCmd, setCopiedCmd] = useState<string | null>(null);

  const copyCmd = (cmd: string) => {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      navigator.clipboard.writeText(cmd);
      setCopiedCmd(cmd);
      setTimeout(() => setCopiedCmd(null), 2000);
    }
  };

  return (
    <section id="download" className="download-section" aria-labelledby="download-heading">
      <div className="container">
        <div className="section-center-head">
          <p className="eyebrow-muted">Desktop Installers</p>
          <h2 id="download-heading">Get UnScanMe for your computer</h2>
          <p>
            Lightweight, self-contained native installers — no cloud sign-in or subscription
            required.
          </p>
        </div>

        <div className="download-grid">
          {PLATFORM_DOWNLOADS.map((plat) => (
            <article key={plat.id} className="download-card">
              <div className="download-card-head">
                <span className="download-icon-badge" aria-hidden="true">
                  <MonitorIcon className="icon-md" />
                </span>
                <div>
                  <h3>{plat.title}</h3>
                  <span className="download-sub">{plat.subtitle}</span>
                </div>
              </div>

              <ol className="download-steps">
                {plat.steps.map((step) => (
                  <li key={step}>{step}</li>
                ))}
              </ol>

              {plat.commandHint && (
                <div className="download-cmd">
                  <code>{plat.commandHint}</code>
                  <button
                    type="button"
                    className="btn-chip"
                    onClick={() => copyCmd(plat.commandHint!)}
                    aria-label="Copy Linux install command"
                  >
                    {copiedCmd === plat.commandHint ? (
                      <CheckIcon className="icon-xs" />
                    ) : (
                      <CopyIcon className="icon-xs" />
                    )}
                  </button>
                </div>
              )}

              <a
                href={plat.href}
                download={plat.fileName}
                className="split-cta split-cta-lime-full"
              >
                <span className="split-cta-label">{plat.buttonLabel}</span>
                <span className="split-cta-icon">
                  <ArrowDownRightIcon className="icon-xs" />
                </span>
              </a>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
