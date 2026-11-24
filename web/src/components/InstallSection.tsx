"use client";

import React, { useState } from "react";
import { PLATFORM_DOWNLOADS } from "../data/showcase-data";
import { CheckIcon, CopyIcon, DownloadIcon, MonitorIcon } from "./Icons";

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
    <section id="download" className="section-block" aria-labelledby="download-heading">
      <div className="container">
        <div className="section-header">
          <p className="section-tag">Download</p>
          <h2 id="download-heading">Get UnScanMe for your desktop</h2>
          <p>
            Native, lightweight installers — download once and launch in two clicks.
          </p>
        </div>

        <div className="grid-3">
          {PLATFORM_DOWNLOADS.map((plat) => (
            <article key={plat.id} className="card download-card">
              <div className="download-head">
                <div className="card-head">
                  <span className="card-icon">
                    <MonitorIcon className="icon-md" />
                  </span>
                  <span className="card-tag">{plat.subtitle}</span>
                </div>
                <h3>{plat.title}</h3>
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
                className="btn btn-primary download-btn"
              >
                <DownloadIcon className="icon-sm" />
                <span>{plat.buttonLabel}</span>
              </a>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
