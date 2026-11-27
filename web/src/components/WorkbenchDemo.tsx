"use client";

import React, { useEffect, useRef, useState } from "react";
import {
  PAGE_PRESETS,
  REFINE_PRESETS,
  SAMPLES,
  type OutputTab,
  type SampleId,
  type TableRowItem,
} from "../data/showcase-data";
import { LogoMarkIcon, SparklesIcon } from "./Icons";
import { WorkbenchOutputPane } from "./WorkbenchOutputPane";

export function WorkbenchDemo() {
  const [sampleId, setSampleId] = useState<SampleId>("invoice");
  const [outputTab, setOutputTab] = useState<OutputTab>("preview");
  const [execMode, setExecMode] = useState<"ollama" | "api">("ollama");
  const [isGenerating, setIsGenerating] = useState<boolean>(false);
  const [deskewed, setDeskewed] = useState<boolean>(false);
  const [cropActive, setCropActive] = useState<boolean>(true);
  const [serifFont, setSerifFont] = useState<boolean>(false);
  const [highlightTotal, setHighlightTotal] = useState<boolean>(true);
  const [tableSelected, setTableSelected] = useState<boolean>(true);
  const [extraRows, setExtraRows] = useState<TableRowItem[]>([]);
  const [refineText, setRefineText] = useState<string>("");
  const [refineLog, setRefineLog] = useState<string>("Ready · 100% offline processing");
  const streamTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    return () => {
      if (streamTimerRef.current) {
        clearTimeout(streamTimerRef.current);
      }
    };
  }, []);

  const sample = SAMPLES[sampleId];
  const allRows = [...sample.rows, ...extraRows];

  const handleSampleChange = (id: SampleId) => {
    setSampleId(id);
    setExtraRows([]);
    setDeskewed(false);
    setCropActive(true);
  };

  const handleSwitchExecMode = (nextMode: "ollama" | "api") => {
    if (isGenerating) return;
    setExecMode(nextMode);
    setRefineLog(
      nextMode === "api"
        ? "Switched to API Key · Gemini 2.5 Flash ready"
        : "Switched to Ollama · 100% offline local mode",
    );
  };

  const cancelGeneration = () => {
    if (streamTimerRef.current) {
      clearTimeout(streamTimerRef.current);
      streamTimerRef.current = null;
    }
    setIsGenerating(false);
    setRefineLog("Generation cancelled.");
  };

  const applyRefinement = (instruction: string) => {
    const trimmed = instruction.trim();
    if (!trimmed || isGenerating) return;

    if (streamTimerRef.current) {
      clearTimeout(streamTimerRef.current);
    }

    setIsGenerating(true);
    setRefineLog(
      execMode === "api"
        ? "Streaming with gemini-2.5-flash (0.3s)…"
        : "Generating with qwen2.5vl:3b locally…",
    );

    const delay = execMode === "api" ? 360 : 760;
    streamTimerRef.current = setTimeout(() => {
      streamTimerRef.current = null;
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
      setIsGenerating(false);
      setRefineLog(
        `Applied "${trimmed}" via ${execMode === "api" ? "Gemini API" : "local Ollama"}`,
      );
      setRefineText("");
    }, delay);
  };

  const paperTilt = deskewed ? 0 : -1.2;

  return (
    <div id="preview" className="workbench-shell" aria-label="UnScanMe interactive preview">
      <div className="app-window">
        <aside className="app-sidebar" aria-label="Workspace navigation">
          <div className="app-sidebar-brand">
            <LogoMarkIcon className="icon-xs lime-dot" />
            <span>UnScanMe Studio</span>
          </div>

          <div className="sidebar-group">
            <span className="sidebar-label">Samples</span>
            {(Object.keys(SAMPLES) as SampleId[]).map((id) => (
              <button
                key={id}
                type="button"
                className={`sidebar-item ${sampleId === id ? "is-active" : ""}`}
                onClick={() => handleSampleChange(id)}
              >
                {SAMPLES[id].shortLabel}
              </button>
            ))}
          </div>

          <div className="sidebar-group">
            <span className="sidebar-label">Output Mode</span>
            <button
              type="button"
              className={`sidebar-item ${outputTab === "preview" ? "is-active" : ""}`}
              onClick={() => setOutputTab("preview")}
            >
              Document
            </button>
            <button
              type="button"
              className={`sidebar-item ${outputTab === "visual" ? "is-active" : ""}`}
              onClick={() => setOutputTab("visual")}
            >
              Table Editor
            </button>
            <button
              type="button"
              className={`sidebar-item ${outputTab === "source" ? "is-active" : ""}`}
              onClick={() => setOutputTab("source")}
            >
              Clean HTML5
            </button>
          </div>

          <div className="sidebar-group">
            <span className="sidebar-label">AI Execution</span>
            <div className="wb-exec-switch" role="group" aria-label="AI execution mode">
              <button
                type="button"
                className={`wb-exec-btn ${execMode === "ollama" ? "is-active" : ""}`}
                aria-pressed={execMode === "ollama"}
                disabled={isGenerating}
                onClick={() => handleSwitchExecMode("ollama")}
              >
                Ollama
              </button>
              <button
                type="button"
                className={`wb-exec-btn ${execMode === "api" ? "is-active" : ""}`}
                aria-pressed={execMode === "api"}
                disabled={isGenerating}
                onClick={() => handleSwitchExecMode("api")}
              >
                API Key
              </button>
            </div>
          </div>

          <div className="sidebar-status">
            <span className="status-dot" aria-hidden="true" />
            <span>{execMode === "api" ? "Gemini · 2.5 Flash API" : "Ollama · qwen2.5vl:3b"}</span>
          </div>
        </aside>

        <div className="app-main">
          <div className="window-chrome">
            <div className="window-title">
              <span>{sample.fileName}</span>
              <span className="window-sub"> · {sample.pdfSize}</span>
            </div>

            <div className="sample-switcher-mobile" role="group" aria-label="Sample document">
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

            <a href="#download" className="window-export-badge">
              Quick Export PDF
            </a>
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
                    {cropActive ? "Crop Box On" : "Crop Box"}
                  </button>
                </div>
              </div>

              <div className="wb-pane-body">
                <div className="scan-surface">
                  <div className="scan-page" style={{ transform: `rotate(${paperTilt}deg)` }}>
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
                    <p className="scan-foot">Drag white box handles to resize capture area</p>
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
                <span>Refine AI</span>
              </span>
              <label htmlFor="refine-demo-input" className="sr-only">
                Refinement instruction
              </label>
              <input
                id="refine-demo-input"
                type="text"
                value={refineText}
                onChange={(e) => setRefineText(e.target.value)}
                placeholder="Ask local AI to tweak layout (e.g., Add warranty row)..."
              />
              <div className="wb-refine-presets">
                {REFINE_PRESETS.map((preset) => (
                  <button
                    key={preset}
                    type="button"
                    className="btn-chip"
                    disabled={isGenerating}
                    onClick={() => applyRefinement(preset)}
                  >
                    {preset}
                  </button>
                ))}
              </div>
              {isGenerating ? (
                <button
                  type="button"
                  className="wb-stop-btn"
                  onClick={cancelGeneration}
                  title="Stop AI generation"
                >
                  Stop
                </button>
              ) : (
                <button type="submit" className="btn-lime-sm">
                  Apply
                </button>
              )}
            </form>
            <p className="wb-refine-status" role="status" aria-live="polite">
              {refineLog}
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
