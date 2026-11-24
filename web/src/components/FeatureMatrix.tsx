import React from "react";
import { CAPABILITIES, WORKFLOW_STEPS } from "../data/showcase-data";
import { CapabilityIcon } from "./Icons";

export function FeatureMatrix() {
  return (
    <>
      <section id="features" className="section-block" aria-labelledby="features-heading">
        <div className="container">
          <div className="section-header">
            <p className="section-tag">Features</p>
            <h2 id="features-heading">Everything you need to turn scans into real documents</h2>
            <p>
              Clean up photos, fix tables visually, refine text with local AI, and export lightweight
              vector PDFs without leaving your desktop.
            </p>
          </div>

          <div className="bento-grid">
            {CAPABILITIES.map((cap) => (
              <article key={cap.id} className={`bento-card ${cap.span}`}>
                <div className="card-head">
                  <span className="card-icon">
                    <CapabilityIcon name={cap.icon} className="icon-md" />
                  </span>
                  <span className="card-tag">{cap.tag}</span>
                </div>
                <h3>{cap.title}</h3>
                <p>{cap.description}</p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section id="how-it-works" className="section-block" aria-labelledby="workflow-heading">
        <div className="container">
          <div className="section-header">
            <p className="section-tag">How it works</p>
            <h2 id="workflow-heading">From messy scan to clean PDF in three steps</h2>
            <p>
              Every document stays on your machine from initial capture to final print-ready export.
            </p>
          </div>

          <ol className="workflow-grid">
            {WORKFLOW_STEPS.map((item) => (
              <li key={item.step} className="workflow-card">
                <span className="workflow-step">{item.step}</span>
                <h3>{item.title}</h3>
                <p>{item.body}</p>
              </li>
            ))}
          </ol>
        </div>
      </section>
    </>
  );
}
