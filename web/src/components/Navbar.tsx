import React from "react";
import { LogoMarkIcon } from "./Icons";

export function Navbar() {
  return (
    <header className="navbar">
      <div className="container navbar-inner">
        <a href="#top" className="brand">
          <span className="brand-badge">
            <LogoMarkIcon className="icon-sm" />
          </span>
          <span className="brand-name">UnScanMe</span>
        </a>

        <nav className="nav-links" aria-label="Primary">
          <a href="#features">Features</a>
          <a href="#how-it-works">How it works</a>
          <a href="#download">Download</a>
          <a href="#faq">FAQ</a>
        </nav>

        <div className="nav-actions">
          <span className="status-chip">
            <span className="status-dot" aria-hidden="true" />
            <span>100% Offline</span>
          </span>
          <a href="#download" className="btn btn-primary btn-sm">
            Download Free
          </a>
        </div>
      </div>
    </header>
  );
}
