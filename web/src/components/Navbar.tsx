"use client";

import React, { useEffect, useRef, useState } from "react";
import { NAV_DOWNLOADS, NAV_FEATURES } from "../data/showcase-data";
import { ArrowDownRightIcon, ChevronDownIcon, CloseIcon, LogoMarkIcon, MenuIcon } from "./Icons";

type DropdownKey = "features" | "downloads" | null;

export function Navbar() {
  const [openMenu, setOpenMenu] = useState<DropdownKey>(null);
  const [mobileOpen, setMobileOpen] = useState(false);
  const [mobileAccordion, setMobileAccordion] = useState<DropdownKey>(null);
  const headerRef = useRef<HTMLElement>(null);

  useEffect(() => {
    const onPointerDown = (e: MouseEvent) => {
      if (headerRef.current && !headerRef.current.contains(e.target as Node)) {
        setOpenMenu(null);
      }
    };
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        setOpenMenu(null);
        setMobileOpen(false);
      }
    };
    document.addEventListener("mousedown", onPointerDown);
    document.addEventListener("keydown", onKeyDown);
    return () => {
      document.removeEventListener("mousedown", onPointerDown);
      document.removeEventListener("keydown", onKeyDown);
    };
  }, []);

  const closeAll = () => {
    setOpenMenu(null);
    setMobileOpen(false);
  };

  return (
    <header ref={headerRef} className="site-header">
      <div className="notch-wrap">
        <div className="notch-bar">
          <a href="#top" className="brand" onClick={closeAll}>
            <LogoMarkIcon className="brand-dot" />
            <span className="brand-name">UnScanMe</span>
          </a>

          <nav className="desktop-nav" aria-label="Primary">
            <div
              className="nav-dropdown-wrap"
              onMouseEnter={() => setOpenMenu("features")}
              onMouseLeave={() => setOpenMenu(null)}
            >
              <button
                type="button"
                className="nav-trigger"
                aria-expanded={openMenu === "features"}
                onClick={() => setOpenMenu((cur) => (cur === "features" ? null : "features"))}
              >
                <span>Features</span>
                <ChevronDownIcon className={`nav-chevron ${openMenu === "features" ? "is-open" : ""}`} />
              </button>
              {openMenu === "features" && (
                <div className="nav-popover" role="menu">
                  {NAV_FEATURES.map((item) => (
                    <a
                      key={item.title}
                      href={item.href}
                      className="nav-popover-item"
                      role="menuitem"
                      onClick={closeAll}
                    >
                      <span className="nav-popover-title">{item.title}</span>
                      <span className="nav-popover-desc">{item.description}</span>
                    </a>
                  ))}
                </div>
              )}
            </div>

            <div
              className="nav-dropdown-wrap"
              onMouseEnter={() => setOpenMenu("downloads")}
              onMouseLeave={() => setOpenMenu(null)}
            >
              <button
                type="button"
                className="nav-trigger"
                aria-expanded={openMenu === "downloads"}
                onClick={() => setOpenMenu((cur) => (cur === "downloads" ? null : "downloads"))}
              >
                <span>Downloads</span>
                <ChevronDownIcon className={`nav-chevron ${openMenu === "downloads" ? "is-open" : ""}`} />
              </button>
              {openMenu === "downloads" && (
                <div className="nav-popover" role="menu">
                  {NAV_DOWNLOADS.map((item) => (
                    <a
                      key={item.title}
                      href={item.href}
                      download={item.download}
                      className="nav-popover-item"
                      role="menuitem"
                      onClick={closeAll}
                    >
                      <span className="nav-popover-title">{item.title}</span>
                      <span className="nav-popover-desc">{item.description}</span>
                    </a>
                  ))}
                </div>
              )}
            </div>

            <a href="#how-it-works" className="nav-link" onClick={closeAll}>
              How it works
            </a>
            <a href="#faq" className="nav-link" onClick={closeAll}>
              FAQ
            </a>
          </nav>

          <div className="nav-cta-group">
            <a href="#preview" className="nav-signin">
              Live Preview
            </a>
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
              <a href="#features" className="mobile-drawer-link" onClick={closeAll}>
                Features overview
              </a>

              <div className="mobile-acc-group">
                <button
                  type="button"
                  className="mobile-drawer-link mobile-acc-btn"
                  aria-expanded={mobileAccordion === "features"}
                  onClick={() =>
                    setMobileAccordion((cur) => (cur === "features" ? null : "features"))
                  }
                >
                  <span>Capabilities</span>
                  <ChevronDownIcon
                    className={`icon-xs ${mobileAccordion === "features" ? "is-open" : ""}`}
                  />
                </button>
                {mobileAccordion === "features" && (
                  <div className="mobile-sublist">
                    {NAV_FEATURES.map((item) => (
                      <a key={item.title} href={item.href} onClick={closeAll}>
                        {item.title}
                      </a>
                    ))}
                  </div>
                )}
              </div>

              <div className="mobile-acc-group">
                <button
                  type="button"
                  className="mobile-drawer-link mobile-acc-btn"
                  aria-expanded={mobileAccordion === "downloads"}
                  onClick={() =>
                    setMobileAccordion((cur) => (cur === "downloads" ? null : "downloads"))
                  }
                >
                  <span>Downloads</span>
                  <ChevronDownIcon
                    className={`icon-xs ${mobileAccordion === "downloads" ? "is-open" : ""}`}
                  />
                </button>
                {mobileAccordion === "downloads" && (
                  <div className="mobile-sublist">
                    {NAV_DOWNLOADS.map((item) => (
                      <a
                        key={item.title}
                        href={item.href}
                        download={item.download}
                        onClick={closeAll}
                      >
                        {item.title}
                      </a>
                    ))}
                  </div>
                )}
              </div>

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
