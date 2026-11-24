"use client";

import React, { useState } from "react";
import {
  PAGE_PRESETS,
  REFINE_PRESETS,
  SAMPLES,
  type OutputTab,
  type SampleId,
  type TableRowItem,
} from "../data/showcase-data";
import { SparklesIcon } from "./Icons";
import { WorkbenchOutputPane } from "./WorkbenchOutputPane";

export function WorkbenchDemo() {
  const [sampleId, setSampleId] = useState<SampleId>("invoice");
  const [outputTab, setOutputTab] = useState<OutputTab>("preview");
  const [deskewed, setDeskewed] = useState<boolean>(false);
  const [cropActive, setCropActive] = useState<boolean>(true);
  const [serifFont, setSerifFont] = useState<boolean>(false);
  const [highlightTotal, setHighlightTotal] = useState<boolean>(true);
  const [tableSelected, setTableSelected] = useState<boolean>(true);
  const [extraRows, setExtraRows] = useState<TableRowItem[]>([]);
  const [refineText, setRefineText] = useState<string>("");
  const [refineLog, setRefineLog] = useState<string>("Ready · 100% offline processing");

  const sample = SAMPLES[sampleId];
  const allRows = [...sample.rows, ...extraRows];

  const handleSampleChange = (id: SampleId) => {
    setSampleId(id);
    setExtraRows([]);
    setDeskewed(false);
    setCropActive(true);
  };

  const applyRefinement = (instruction: string) => {
    const trimmed = instruction.trim();
    if (!trimmed) return;
    const lower = trimmed.toLowerCase();
    if (lower.includes("serif")) {
      setSerifFont(true);
    } else if (lower.includes("sans")) {
      setSerifFont(false);
    }
    if (lower.includes("row") || lower.includes("warranty") || lower.includes("discount")) {
      setExtraRows((prev) => [
        ...prev,
        { col1: "Added via Refine AI", col2: "1 item", col3: "Included" },
      ]);
    }
    if (lower.includes("highlight") || lower.includes("emerald")) {
      setHighlightTotal(true);
    }
    setRefineLog(`Applied "${trimmed}" locally`);
    setRefineText("");
  };

  const paperTilt = deskewed ? 0 : -1.3;

  return (
    <section id="preview" className="preview-section" aria-labelledby="preview-heading">
      <div className="container">
        <h2 id="preview-heading" className="sr-only">
          Interactive Product Preview
        </h2>

        <div className="app-window">
          <div className="window-chrome">
            <div className="window-dots" aria-hidden="true">
              <span />
              <span />
              <span />
            </div>

            <div className="window-title">
              UnScanMe — <span>{sample.fileName}</span> · Local AI
            </div>

            <div className="sample-switcher" role="group" aria-label="Sample document">
              {(Object.keys(SAMPLES) as SampleId[]).map((id) => (
                <button
                  key={id}
                  type="button"
                  className={`btn-chip ${sampleId === id ? "is-active" : ""}`}
                  onClick={() => handleSampleChange(id)}
                >
                  {SAMPLES[id].shortLabel}
                </button>
              ))}
            </div>
          </div>

          <div className="wb-grid">
            <div className="wb-pane">
              <div className="wb-pane-bar">
                <span>Original Scan ({sample.scanSize})</span>
                <div className="wb-toolbar-group">
                  <button
                    type="button"
                    className={`btn-chip ${deskewed ? "is-selected" : ""}`}
                    onClick={() => setDeskewed((v) => !v)}
                  >
                    {deskewed ? "Deskewed 0.0°" : "Auto-Deskew"}
                  </button>
                  <button
                    type="button"
                    className={`btn-chip ${cropActive ? "is-selected" : ""}`}
                    onClick={() => setCropActive((v) => !v)}
                  >
                    {cropActive ? "Crop Region On" : "Crop Region"}
                  </button>
                </div>
              </div>

              <div className="wb-pane-body">
                <div className="scan-surface">
                  <div
                    className="scan-page"
                    style={{ transform: `rotate(${paperTilt}deg)` }}
                  >
                    {cropActive && (
                      <div className="scan-crop-frame" aria-hidden="true">
                        <span className="crop-handle h-nw" />
                        <span className="crop-handle h-n" />
                        <span className="crop-handle h-ne" />
                        <span className="crop-handle h-e" />
                        <span className="crop-handle h-se" />
                        <span className="crop-handle h-s" />
                        <span className="crop-handle h-sw" />
                        <span className="crop-handle h-w" />
                      </div>
                    )}
                    <h4>{sample.heading}</h4>
                    <p className="scan-sub">{sample.subtitle}</p>
                    <table className="scan-table">
                      <thead>
                        <tr>
                          <th>{sample.headers[0]}</th>
                          <th>{sample.headers[1]}</th>
                          <th>{sample.headers[2]}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {sample.rows.map((r) => (
                          <tr key={r.col1}>
                            <td>{r.col1}</td>
                            <td>{r.col2}</td>
                            <td>{r.col3}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                    <p className="scan-foot">Paste with Ctrl+V or drag &amp; drop</p>
                  </div>
                </div>
              </div>
            </div>

            <WorkbenchOutputPane
              sample={sample}
              rows={allRows}
              outputTab={outputTab}
              onSelectTab={setOutputTab}
              pagePreset={PAGE_PRESETS["a4-portrait"]}
              serifFont={serifFont}
              highlightTotal={highlightTotal}
              tableSelected={tableSelected}
              onToggleTableSelect={() => setTableSelected((v) => !v)}
              onAddRow={() =>
                setExtraRows((prev) => [
                  ...prev,
                  { col1: `New Line Item #${allRows.length + 1}`, col2: "1 unit", col3: "$150.00" },
                ])
              }
              onDeleteLastRow={() => setExtraRows((prev) => prev.slice(0, -1))}
            />
          </div>

          <div className="wb-refine-bar">
            <form
              className="wb-refine-form"
              onSubmit={(e) => {
                e.preventDefault();
                applyRefinement(refineText);
              }}
            >
              <span className="wb-refine-label">
                <SparklesIcon className="icon-xs" />
                <span>Ask AI to refine</span>
              </span>
              <label htmlFor="refine-demo-input" className="sr-only">
                Refinement instruction
              </label>
              <input
                id="refine-demo-input"
                type="text"
                value={refineText}
                onChange={(e) => setRefineText(e.target.value)}
                placeholder="Ask AI to adjust formatting (e.g., Add warranty row)..."
              />
              <div className="wb-refine-presets">
                {REFINE_PRESETS.map((preset) => (
                  <button
                    key={preset}
                    type="button"
                    className="btn-chip"
                    onClick={() => applyRefinement(preset)}
                  >
                    {preset}
                  </button>
                ))}
              </div>
              <button type="submit" className="btn btn-primary btn-sm">
                Apply
              </button>
            </form>
            <p className="wb-refine-status" role="status" aria-live="polite">
              {refineLog}
            </p>
          </div>
        </div>
      </div>
    </section>
  );
}
