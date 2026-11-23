"use client";

import React, { useState } from "react";
import {
  PAGE_PRESETS,
  REFINE_PRESETS,
  SAMPLES,
  WORKBENCH_MODES,
  type OutputTab,
  type PagePresetId,
  type SampleId,
  type TableRowItem,
  type WorkbenchMode,
} from "../data/showcase-data";
import { WorkbenchOutputPane } from "./WorkbenchOutputPane";

export function WorkbenchDemo() {
  const [mode, setMode] = useState<WorkbenchMode>("image");
  const [sampleId, setSampleId] = useState<SampleId>("invoice");
  const [outputTab, setOutputTab] = useState<OutputTab>("preview");
  const [pagePresetId, setPagePresetId] = useState<PagePresetId>("a4-portrait");
  const [deskewed, setDeskewed] = useState<boolean>(false);
  const [rotatedDeg, setRotatedDeg] = useState<number>(0);
  const [cropped, setCropped] = useState<boolean>(false);
  const [serifFont, setSerifFont] = useState<boolean>(false);
  const [highlightTotal, setHighlightTotal] = useState<boolean>(true);
  const [tableSelected, setTableSelected] = useState<boolean>(true);
  const [extraRows, setExtraRows] = useState<TableRowItem[]>([]);
  const [refineText, setRefineText] = useState<string>("");
  const [refineLog, setRefineLog] = useState<string>("Ready for targeted AI follow-up edits.");

  const sample = SAMPLES[sampleId];
  const allRows = [...sample.rows, ...extraRows];

  const handleSampleChange = (id: SampleId) => {
    setSampleId(id);
    setExtraRows([]);
    setDeskewed(false);
    setRotatedDeg(0);
    setCropped(false);
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
    setRefineLog(`Applied: "${trimmed}" (inline data:image URIs protected)`);
    setRefineText("");
  };

  const paperTilt = deskewed ? 0 : -1.2;
  const transformCss = `rotate(${rotatedDeg + paperTilt}deg) scale(${cropped ? 1.03 : 1})`;

  return (
    <section id="workbench" className="section-block" aria-labelledby="workbench-heading">
      <div className="container">
        <div className="section-header">
          <p className="section-tag">Interactive Workbench</p>
          <h2 id="workbench-heading">Built Around Your Document Pipeline</h2>
          <p>
            Switch input modes, test pure-Rust deskew and crop controls, inspect the Visual Table
            Editor, or apply conversational edits with Refine AI.
          </p>
        </div>

        <div className="workbench">
          <div className="wb-toolbar">
            <div className="wb-toolbar-group" role="group" aria-label="Workbench mode">
              {WORKBENCH_MODES.map((m) => (
                <button
                  key={m.id}
                  type="button"
                  className={`btn-chip ${mode === m.id ? "is-active" : ""}`}
                  onClick={() => setMode(m.id)}
                >
                  {m.label}
                </button>
              ))}
            </div>

            <div className="wb-toolbar-group" role="group" aria-label="Page setup preset">
              {(Object.keys(PAGE_PRESETS) as PagePresetId[]).map((pid) => (
                <button
                  key={pid}
                  type="button"
                  className={`btn-chip ${pagePresetId === pid ? "is-active" : ""}`}
                  onClick={() => setPagePresetId(pid)}
                >
                  {PAGE_PRESETS[pid].label}
                </button>
              ))}
            </div>
          </div>

          <div className="wb-subbar">
            <div className="wb-toolbar-group" role="group" aria-label="Sample document">
              {(Object.keys(SAMPLES) as SampleId[]).map((id) => (
                <button
                  key={id}
                  type="button"
                  className={`btn-chip ${sampleId === id ? "is-active" : ""}`}
                  onClick={() => handleSampleChange(id)}
                >
                  {SAMPLES[id].title}
                </button>
              ))}
            </div>
            <span className="wb-status-pill" aria-live="polite">
              Model: <code>qwen2.5vl:3b</code> · Local Ollama Ready
            </span>
          </div>

          <div className="wb-grid">
            <div className="wb-pane">
              <div className="wb-pane-bar">
                <span>
                  {mode === "pdf"
                    ? `PDF Source · ${sample.fileName.replace(/\.\w+$/, ".pdf")} (${sample.pdfPages} pages)`
                    : mode === "prompt"
                      ? "Prompt to HTML5 Brief"
                      : `Image Source · ${sample.fileName}`}
                </span>
                {mode === "image" && (
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
                      className="btn-chip"
                      onClick={() => setRotatedDeg((d) => (d + 90) % 360)}
                    >
                      Rotate 90°
                    </button>
                    <button
                      type="button"
                      className={`btn-chip ${cropped ? "is-selected" : ""}`}
                      onClick={() => setCropped((v) => !v)}
                    >
                      {cropped ? "Cropped" : "Crop"}
                    </button>
                  </div>
                )}
              </div>

              <div className="wb-pane-body">
                {mode === "prompt" ? (
                  <div className="prompt-stage">
                    <p className="prompt-stage-label">Active Generation Brief</p>
                    <p className="prompt-stage-text">
                      Create an editable HTML5 {sample.title.toLowerCase()} with semantic{" "}
                      <code>&lt;table&gt;</code> rows, print-safe{" "}
                      <code>@page &#123; size: {PAGE_PRESETS[pagePresetId].cssSize}; &#125;</code>,
                      and itemized totals for {sample.heading}.
                    </p>
                    <p className="prompt-stage-hint">
                      Tip: Output enters Preview and unlocks Visual Edit + Table Editor immediately.
                    </p>
                  </div>
                ) : (
                  <div className="scan-surface">
                    <div className="scan-page" style={{ transform: transformCss }}>
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
                      <p className="scan-foot">
                        {mode === "pdf"
                          ? "Extracted via Poppler -bbox-layout X/Y clustering"
                          : "Clipboard Ctrl+V / Drag-and-Drop supported"}
                      </p>
                    </div>
                  </div>
                )}
              </div>
            </div>

            <WorkbenchOutputPane
              sample={sample}
              rows={allRows}
              outputTab={outputTab}
              onSelectTab={setOutputTab}
              pagePreset={PAGE_PRESETS[pagePresetId]}
              serifFont={serifFont}
              highlightTotal={highlightTotal}
              tableSelected={tableSelected}
              onToggleTableSelect={() => setTableSelected((v) => !v)}
              onAddRow={() =>
                setExtraRows((prev) => [
                  ...prev,
                  { col1: `New Item #${allRows.length + 1}`, col2: "1 unit", col3: "$150.00" },
                ])
              }
              onDeleteLastRow={() => setExtraRows((prev) => prev.slice(0, -1))}
            />
          </div>

          <div className="wb-refine-bar">
            <div className="wb-refine-row">
              <span className="wb-refine-title">Refine with AI</span>
              <div className="wb-toolbar-group">
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
            </div>
            <form
              className="wb-refine-form"
              onSubmit={(e) => {
                e.preventDefault();
                applyRefinement(refineText);
              }}
            >
              <label htmlFor="refine-demo-input" className="sr-only">
                Refinement instruction
              </label>
              <input
                id="refine-demo-input"
                type="text"
                value={refineText}
                onChange={(e) => setRefineText(e.target.value)}
                placeholder="Ask AI to refine the generated HTML (e.g., Add a discount row)..."
              />
              <button type="submit" className="btn btn-primary btn-sm">
                Apply Refinement
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
