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
          <span className="brand-text">
            <span className="brand-name">DocForge</span>
            <span className="brand-sub">Local Document Workbench</span>
          </span>
        </a>

        <nav className="nav-links" aria-label="Primary">
          <a href="#workbench">Workbench</a>
          <a href="#capabilities">Capabilities</a>
          <a href="#pipeline">Architecture</a>
          <a href="#install">Setup</a>
          <a href="#faq">FAQ</a>
        </nav>

        <div className="nav-actions">
          <span className="status-chip">
            <span className="status-dot" aria-hidden="true" />
            <span>100% Local</span>
          </span>
          <a href="#install" className="btn btn-primary btn-sm">
            Get Desktop App
          </a>
        </div>
      </div>
    </header>
  );
}
