"use client";

import React, { useEffect, useRef, useState } from "react";
import { STATEMENT_TEXT } from "../data/showcase-data";

const WORDS = STATEMENT_TEXT.split(" ");

export function StatementReveal() {
  const sectionRef = useRef<HTMLElement>(null);
  const [litRatio, setLitRatio] = useState<number>(0.42);

  useEffect(() => {
    const reduced =
      typeof window !== "undefined" &&
      window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    if (reduced) {
      setLitRatio(1);
      return;
    }

    const update = () => {
      const el = sectionRef.current;
      if (!el) return;
      const rect = el.getBoundingClientRect();
      const vh = window.innerHeight || 900;
      const raw = (vh * 0.85 - rect.top) / (vh * 0.55 + rect.height * 0.45);
      const clamped = Math.max(0.25, Math.min(1, raw));
      setLitRatio(clamped);
    };

    update();
    window.addEventListener("scroll", update, { passive: true });
    window.addEventListener("resize", update, { passive: true });
    return () => {
      window.removeEventListener("scroll", update);
      window.removeEventListener("resize", update);
    };
  }, []);

  const litCount = Math.round(litRatio * WORDS.length);

  return (
    <section ref={sectionRef} className="statement-section" aria-label="Product philosophy">
      <div className="container statement-inner">
        <p className="statement-copy">
          {WORDS.map((word, idx) => (
            <span
              key={`${word}-${idx}`}
              className={`statement-word ${idx < litCount ? "is-lit" : ""}`}
            >
              {word}{" "}
            </span>
          ))}
        </p>
      </div>
    </section>
  );
}
