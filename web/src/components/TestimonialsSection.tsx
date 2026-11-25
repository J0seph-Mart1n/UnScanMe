"use client";

import React, { useState } from "react";
import { TESTIMONIALS } from "../data/showcase-data";

export function TestimonialsSection() {
  const [activeIdx, setActiveIdx] = useState<number>(0);
  const current = TESTIMONIALS[activeIdx];

  return (
    <section className="testimonials-section" aria-labelledby="testimonials-heading">
      <div className="container">
        <h2 id="testimonials-heading" className="testimonials-title">
          Built for workflows everywhere
        </h2>

        <div className="testimonials-body">
          <div className="testimonials-avatars" role="tablist" aria-label="Select workflow story">
            {TESTIMONIALS.map((item, idx) => {
              const selected = idx === activeIdx;
              return (
                <button
                  key={item.id}
                  type="button"
                  role="tab"
                  aria-selected={selected}
                  aria-label={`${item.name}, ${item.org}`}
                  className={`avatar-ring-btn ${selected ? "is-active" : ""}`}
                  onClick={() => setActiveIdx(idx)}
                >
                  <span className="avatar-monogram">{item.initials}</span>
                </button>
              );
            })}
          </div>

          <div className="testimonials-quote-wrap" role="tabpanel" aria-live="polite">
            <blockquote className="testimonials-quote">
              &ldquo;{current.quote}&rdquo;
            </blockquote>
            <p className="testimonials-author">
              <strong>{current.name}</strong>,{" "}
              <span>
                {current.role} @ {current.org}
              </span>
            </p>
          </div>
        </div>

        <div className="testimonials-brands" role="group" aria-label="Workflow teams">
          {TESTIMONIALS.map((item, idx) => (
            <button
              key={item.id}
              type="button"
              className={`brand-tab ${idx === activeIdx ? "is-active" : ""}`}
              onClick={() => setActiveIdx(idx)}
            >
              <span className="brand-tab-glyph" aria-hidden="true" />
              <span>{item.category}</span>
            </button>
          ))}
        </div>
      </div>
    </section>
  );
}
