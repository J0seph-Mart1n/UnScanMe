"use client";

import React, { useState } from "react";
import { PLATFORM_DOWNLOADS } from "../data/showcase-data";
import { CheckIcon, CopyIcon } from "./Icons";

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
    <section id="install" className="section-block" aria-labelledby="install-heading">
      <div className="container">
        <div className="section-header">
          <p className="section-tag">Download</p>
          <h2 id="install-heading">Download & Install in Two Clicks</h2>
          <p>
            Pick your operating system, download the installer, and launch it directly.
          </p>
        </div>

        <div className="grid-3">
          {PLATFORM_DOWNLOADS.map((plat) => (
            <article key={plat.id} className="card download-card">
              <div className="download-head">
                <h3>{plat.title}</h3>
                <span className="card-tag">{plat.subtitle}</span>
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
                {plat.buttonLabel}
              </a>
            </article>
          ))}
        </div>
      </div>
    </section>
  );
}
