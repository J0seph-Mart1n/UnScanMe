import React from "react";
import { CAPABILITIES, PIPELINE_STEPS } from "../data/showcase-data";
import { CapabilityIcon } from "./Icons";

export function FeatureMatrix() {
  return (
    <>
      <section id="capabilities" className="section-block" aria-labelledby="capabilities-heading">
        <div className="container">
          <div className="section-header">
            <p className="section-tag">Workbench Capabilities</p>
            <h2 id="capabilities-heading">Six Integrated Tools in One Local Desktop App</h2>
            <p>
              Built in Rust and Dioxus 0.7 with a pure-Rust image preprocessor, interactive DOM
              canvas, Poppler PDF extractor, and local Ollama integration.
            </p>
          </div>

          <div className="grid-3">
            {CAPABILITIES.map((cap) => (
              <article key={cap.id} className="card">
                <div className="card-head">
                  <span className="card-icon">
                    <CapabilityIcon name={cap.icon} className="icon-md" />
                  </span>
                  <span className="card-tag">{cap.tag}</span>
                </div>
                <h3>{cap.title}</h3>
                <p>{cap.description}</p>
                <p className="card-meta">{cap.detail}</p>
              </article>
            ))}
          </div>
        </div>
      </section>

      <section id="pipeline" className="section-block" aria-labelledby="pipeline-heading">
        <div className="container">
          <div className="section-header">
            <p className="section-tag">Local Architecture</p>
            <h2 id="pipeline-heading">Deterministic From Input to Searchable PDF</h2>
            <p>
              Every document stays on your disk and moves through an inspectable HTML5 stage before
              printing.
            </p>
          </div>

          <ol className="pipeline-grid">
            {PIPELINE_STEPS.map((item) => (
              <li key={item.step} className="pipeline-card">
                <span className="pipeline-step">{item.step}</span>
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
