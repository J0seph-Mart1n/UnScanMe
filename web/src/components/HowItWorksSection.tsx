"use client";

import React, { useEffect, useRef, useState } from "react";
import { WORKFLOW_STEPS } from "../data/showcase-data";
import { PasteIcon, RocketIcon, TableIcon } from "./Icons";

function StepIcon({ icon }: { icon: "paste" | "table" | "rocket" }) {
  switch (icon) {
    case "paste":
      return <PasteIcon className="icon-md" />;
    case "table":
      return <TableIcon className="icon-md" />;
    case "rocket":
      return <RocketIcon className="icon-md" />;
  }
}

export function HowItWorksSection() {
  const sectionRef = useRef<HTMLElement>(null);
  const listRef = useRef<HTMLOListElement>(null);
  const [progress, setProgress] = useState<number>(36);
  const [line, setLine] = useState<{ top: number; height: number } | null>(null);

  useEffect(() => {
    const measureLine = () => {
      const listEl = listRef.current;
      if (!listEl) return;
      const icons = listEl.querySelectorAll<HTMLElement>(".timeline-icon");
      if (icons.length < 2) return;
      const listRect = listEl.getBoundingClientRect();
      const firstRect = icons[0].getBoundingClientRect();
      const lastRect = icons[icons.length - 1].getBoundingClientRect();
      const firstCenterY = firstRect.top - listRect.top + firstRect.height / 2;
      const lastCenterY = lastRect.top - listRect.top + lastRect.height / 2;
      setLine({
        top: Math.round(firstCenterY),
        height: Math.max(0, Math.round(lastCenterY - firstCenterY)),
      });
    };

    const reduced =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) {
      setProgress(100);
      measureLine();
      window.addEventListener("resize", measureLine, { passive: true });
      return () => {
        window.removeEventListener("resize", measureLine);
      };
    }

    const update = () => {
      measureLine();
      const el = sectionRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const vh = window.innerHeight || 900;
      const raw = (vh * 0.65 - rect.top) / Math.max(260, rect.height * 0.72);
      setProgress(Math.round(Math.max(15, Math.min(100, raw * 100))));
    };

    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update, { passive: true });
    return () => {
      window.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, []);

  return (
    <section
      id="how-it-works"
      ref={sectionRef}
      className="how-section"
      aria-labelledby="how-heading"
    >
      <div className="container how-grid">
        <div className="how-left">
          <div className="how-sticky">
            <h2 id="how-heading">How it works</h2>
            <p>
              Native desktop installers for Windows, macOS, and Linux — built around your local
              workflow from initial capture to searchable PDF.
            </p>
            <a href="#download" className="btn-white-pill">
              Download Free
            </a>
          </div>
        </div>

        <ol ref={listRef} className="how-timeline">
          <div
            className="timeline-track"
            style={line ? { top: line.top, height: line.height, bottom: "auto" } : undefined}
            aria-hidden="true"
          >
            <div className="timeline-fill" style={{ height: `${progress}%` }} />
          </div>
          {WORKFLOW_STEPS.map((item) => (
            <li key={item.step} className="timeline-item">
              <span className="timeline-icon" aria-hidden="true">
                <StepIcon icon={item.icon} />
              </span>
              <div className="timeline-content">
                <h3>{item.title}</h3>
                <p>{item.body}</p>
              </div>
            </li>
          ))}
        </ol>
      </div>
    </section>
  );
}
