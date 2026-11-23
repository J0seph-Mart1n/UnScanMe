"use client";

import React, { useRef } from "react";
import type { OutputTab, PagePreset, SampleDoc, TableRowItem } from "../data/showcase-data";

interface WorkbenchOutputPaneProps {
  sample: SampleDoc;
  rows: TableRowItem[];
  outputTab: OutputTab;
  onSelectTab: (tab: OutputTab) => void;
  pagePreset: PagePreset;
  serifFont: boolean;
  highlightTotal: boolean;
  tableSelected: boolean;
  onToggleTableSelect: () => void;
  onAddRow: () => void;
  onDeleteLastRow: () => void;
}

const OUTPUT_TABS: { id: OutputTab; label: string }[] = [
  { id: "preview", label: "Preview" },
  { id: "visual", label: "Visual Edit" },
  { id: "source", label: "HTML source" },
];

export function WorkbenchOutputPane({
  sample,
  rows,
  outputTab,
  onSelectTab,
  pagePreset,
  serifFont,
  highlightTotal,
  tableSelected,
  onToggleTableSelect,
  onAddRow,
  onDeleteLastRow,
}: WorkbenchOutputPaneProps) {
  const tabRefs = useRef<(HTMLButtonElement | null)[]>([]);

  const generatedHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <style id="__df_page_setup__">
    @page { size: ${pagePreset.cssSize}; margin: ${pagePreset.marginMm}mm; }
  </style>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body { font-family: ${serifFont ? "Georgia, serif" : "'Inter', system-ui, sans-serif"}; color: #0f172a; }
    table { width: 100%; border-collapse: collapse; margin: 14px 0; }
    th, td { border: 1px solid #cbd5e1; padding: 7px 10px; text-align: left; }
    th { background: #f1f5f9; }
    .total-row { ${highlightTotal ? "background: #dcfce7; color: #166534; font-weight: 700;" : "font-weight: 700;"} }
  </style>
</head>
<body>
  <h1>${sample.heading}</h1>
  <p>${sample.subtitle}</p>
  <table data-df-kind="table">
    <thead>
      <tr><th>${sample.headers[0]}</th><th>${sample.headers[1]}</th><th>${sample.headers[2]}</th></tr>
    </thead>
    <tbody>
${rows.map((r) => `      <tr><td>${r.col1}</td><td>${r.col2}</td><td>${r.col3}</td></tr>`).join("\n")}
      <tr class="total-row"><td colspan="2">${sample.totalLabel}</td><td>${sample.totalValue}</td></tr>
    </tbody>
  </table>
  <p><small>${sample.note}</small></p>
</body>
</html>`;

  const handleTabKeyDown = (e: React.KeyboardEvent<HTMLButtonElement>, index: number) => {
    if (e.key !== "ArrowRight" && e.key !== "ArrowLeft") return;
    e.preventDefault();
    const next =
      e.key === "ArrowRight"
        ? (index + 1) % OUTPUT_TABS.length
        : (index - 1 + OUTPUT_TABS.length) % OUTPUT_TABS.length;
    onSelectTab(OUTPUT_TABS[next].id);
    tabRefs.current[next]?.focus();
  };

  return (
    <div className="wb-pane">
      <div className="wb-pane-bar">
        <div className="wb-tabs" role="tablist" aria-label="Document output view">
          {OUTPUT_TABS.map((t, idx) => (
            <button
              key={t.id}
              ref={(el) => {
                tabRefs.current[idx] = el;
              }}
              id={`wb-tab-${t.id}`}
              type="button"
              role="tab"
              aria-selected={outputTab === t.id}
              aria-controls={`wb-panel-${t.id}`}
              tabIndex={outputTab === t.id ? 0 : -1}
              className={`wb-tab ${outputTab === t.id ? "is-active" : ""}`}
              onClick={() => onSelectTab(t.id)}
              onKeyDown={(e) => handleTabKeyDown(e, idx)}
            >
              {t.label}
            </button>
          ))}
        </div>
        <span className="wb-meta-tag">{pagePreset.label}</span>
      </div>

      {outputTab === "visual" && (
        <div className="wb-editor-strip" role="toolbar" aria-label="Interactive table controls">
          <button type="button" className="btn-chip" onClick={onAddRow}>
            + Row
          </button>
          <button type="button" className="btn-chip" onClick={onDeleteLastRow} disabled={rows.length <= 1}>
            - Row
          </button>
          <button
            type="button"
            className={`btn-chip ${tableSelected ? "is-selected" : ""}`}
            onClick={onToggleTableSelect}
          >
            {tableSelected ? "Table Selected (Drag Ready)" : "Select Table"}
          </button>
        </div>
      )}

      <div
        id={`wb-panel-${outputTab}`}
        role="tabpanel"
        aria-labelledby={`wb-tab-${outputTab}`}
        className="wb-pane-body"
      >
        {outputTab === "source" ? (
          <pre className="code-view">{generatedHtml}</pre>
        ) : (
          <div className={`pdf-sheet ${serifFont ? "is-serif" : ""}`}>
            <div className="pdf-sheet-head">
              <div>
                <h3>{sample.heading}</h3>
                <p className="pdf-sheet-sub">{sample.subtitle}</p>
              </div>
              <span className="pdf-badge">{sample.pdfSize}</span>
            </div>

            <div className={`pdf-table-wrap ${outputTab === "visual" && tableSelected ? "is-df-selected" : ""}`}>
              {outputTab === "visual" && tableSelected && (
                <span className="df-move-pill" aria-hidden="true">
                  ⠿ Move Table
                </span>
              )}
              <table>
                <thead>
                  <tr>
                    <th>{sample.headers[0]}</th>
                    <th>{sample.headers[1]}</th>
                    <th>{sample.headers[2]}</th>
                  </tr>
                </thead>
                <tbody>
                  {rows.map((r, i) => (
                    <tr key={`${r.col1}-${i}`}>
                      <td>{r.col1}</td>
                      <td>{r.col2}</td>
                      <td>{r.col3}</td>
                    </tr>
                  ))}
                  <tr className={highlightTotal ? "highlight-total" : "plain-total"}>
                    <td colSpan={2}>{sample.totalLabel}</td>
                    <td>{sample.totalValue}</td>
                  </tr>
                </tbody>
              </table>
            </div>

            <p className="pdf-sheet-note">{sample.note}</p>
          </div>
        )}
      </div>
    </div>
  );
}
