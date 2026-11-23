"use client";

import React, { useRef, useState } from "react";
import { OS_COMMANDS, type OsTab } from "../data/showcase-data";
import { CheckIcon, CopyIcon } from "./Icons";

const OS_KEYS: { id: OsTab; label: string }[] = [
  { id: "linux", label: "Linux" },
  { id: "macos", label: "macOS" },
  { id: "windows", label: "Windows" },
];

export function InstallSection() {
  const [osTab, setOsTab] = useState<OsTab>("linux");
  const [copied, setCopied] = useState<boolean>(false);
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const handleCopy = () => {
    if (typeof navigator !== "undefined" && navigator.clipboard) {
      navigator.clipboard.writeText(OS_COMMANDS[osTab].commands);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleTabKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const next =
      e.key === "ArrowRight"
        ? (index + 1) % OS_KEYS.length
        : (index - 1 + OS_KEYS.length) % OS_KEYS.length;
    setOsTab(OS_KEYS[next].id);
    tabRefs.current[next]?.focus();
  };

  const active = OS_COMMANDS[osTab];

  return (
    <section id="install" className="section-block" aria-labelledby="install-heading">
      <div className="container">
        <div className="section-header">
          <p className="section-tag">Cross-Platform Desktop</p>
          <h2 id="install-heading">Run Natively on Linux, macOS & Windows</h2>
          <p>
            Zero Electron overhead. Automatically locates Chromium, Chrome, Brave, or Microsoft
            Edge plus system clipboard and Poppler helpers.
          </p>
        </div>

        <div className="install-card">
          <div className="install-top">
            <div className="wb-tabs" role="tablist" aria-label="Operating system setup">
              {OS_KEYS.map((item, idx) => (
                <button
                  key={item.id}
                  ref={(el) => {
                    tabRefs.current[idx] = el;
                  }}
                  id={`os-tab-${item.id}`}
                  type="button"
                  role="tab"
                  aria-selected={osTab === item.id}
                  aria-controls={`os-panel-${item.id}`}
                  tabIndex={osTab === item.id ? 0 : -1}
                  className={`wb-tab ${osTab === item.id ? "is-active" : ""}`}
                  onClick={() => setOsTab(item.id)}
                  onKeyDown={(e) => handleTabKeyDown(e, idx)}
                >
                  {item.label}
                </button>
              ))}
            </div>
            <span className="status-chip">{active.badge}</span>
          </div>

          <div
            id={`os-panel-${osTab}`}
            role="tabpanel"
            aria-labelledby={`os-tab-${osTab}`}
            className="install-panel"
          >
            <div>
              <h3>{active.label}</h3>
              <p>{active.browserNote}</p>
            </div>

            <div className="terminal-wrap">
              <pre className="terminal-box">{active.commands}</pre>
              <button
                type="button"
                className="btn btn-sm terminal-copy"
                onClick={handleCopy}
              >
                {copied ? <CheckIcon className="icon-xs" /> : <CopyIcon className="icon-xs" />}
                <span>{copied ? "Copied" : "Copy"}</span>
              </button>
              <span className="sr-only" role="status" aria-live="polite">
                {copied ? "Install commands copied to clipboard." : ""}
              </span>
            </div>
          </div>
        </div>
      </div>
    </section>
  );
}
