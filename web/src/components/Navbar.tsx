"use client";

import React, { useEffect, useState } from "react";
import { ArrowDownRightIcon, CloseIcon, LogoMarkIcon, MenuIcon } from "./Icons";

export function Navbar() {
  const [mobileOpen, setMobileOpen] = useState(false);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setMobileOpen(false);
      }
    };
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const closeAll = () => {
    setMobileOpen(false);
  };

  return (
    <header className="site-header">
      <div className="notch-wrap">
        <div className="notch-bar">
          <a href="#top" className="brand" onClick={closeAll}>
            <LogoMarkIcon className="brand-dot" />
            <span className="brand-name">UnScanMe</span>
          </a>

          <nav className="desktop-nav" aria-label="Primary">
            <a href="#how-it-works" className="nav-link" onClick={closeAll}>
              How it works
            </a>
            <a href="#faq" className="nav-link" onClick={closeAll}>
              FAQ
            </a>
            <a href="#preview" className="nav-link" onClick={closeAll}>
              Live Preview
            </a>
          </nav>

          <div className="nav-cta-group">
            <a href="#download" className="split-cta split-cta-light-nav">
              <span className="split-cta-label">Download Free</span>
              <span className="split-cta-icon">
                <ArrowDownRightIcon className="icon-xs" />
              </span>
            </a>
          </div>

          <button
            type="button"
            className="mobile-menu-btn"
            aria-label={mobileOpen ? "Close navigation menu" : "Open navigation menu"}
            aria-expanded={mobileOpen}
            onClick={() => setMobileOpen((v) => !v)}
          >
            {mobileOpen ? <CloseIcon className="icon-md" /> : <MenuIcon className="icon-md" />}
          </button>
        </div>

        {mobileOpen && (
          <div className="mobile-drawer" role="dialog" aria-label="Mobile navigation">
            <div className="mobile-drawer-list">
              <a href="#how-it-works" className="mobile-drawer-link" onClick={closeAll}>
                How it works
              </a>
              <a href="#faq" className="mobile-drawer-link" onClick={closeAll}>
                FAQ
              </a>
            </div>

            <div className="mobile-drawer-actions">
              <a href="#preview" className="mobile-preview-link" onClick={closeAll}>
                Live Preview
              </a>
              <a href="#download" className="split-cta split-cta-light-nav" onClick={closeAll}>
                <span className="split-cta-label">Download Free</span>
                <span className="split-cta-icon">
                  <ArrowDownRightIcon className="icon-xs" />
                </span>
              </a>
            </div>
          </div>
        )}
      </div>
    </header>
  );
}
