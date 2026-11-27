// UnScanMe Visual Canvas Editor Controller
// Mounted inside the main Dioxus WebView via document::eval and drives #visual-editor-frame.
(function () {
  const FRAME_ID = "visual-editor-frame";
  const UI_ID = "__df_editor_ui__";
  const STYLE_ID = "__df_editor_style__";
  const MAX_HISTORY = 50;
  const GRID_SNAP = 4;

  const HANDLES = ["nw", "n", "ne", "e", "se", "s", "sw", "w"];
  const VOID_TAGS = new Set(["IMG", "HR", "BR", "INPUT", "SVG", "PATH", "LINE", "RECT", "CIRCLE", "POLYGON"]);

  function rgbToHex(input) {
    if (!input || input === "transparent") return "#000000";
    if (input.startsWith("#")) {
      if (input.length === 4) {
        return "#" + input[1] + input[1] + input[2] + input[2] + input[3] + input[3];
      }
      return input.slice(0, 7);
    }
    const m = input.match(/rgba?\(\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)(?:\s*,\s*([\d.]+))?/i);
    if (!m) return "#000000";
    if (m[4] !== undefined && parseFloat(m[4]) === 0) return "transparent";
    const hex = (n) => Number(n).toString(16).padStart(2, "0");
    return "#" + hex(m[1]) + hex(m[2]) + hex(m[3]);
  }

  function snap(val, free) {
    return free ? Math.round(val) : Math.round(val / GRID_SNAP) * GRID_SNAP;
  }

  function isEditorNode(el) {
    return !!(el && el.closest && el.closest("[data-df-editor-ui='1']"));
  }

  function elementKind(el) {
    if (!el) return "element";
    const k = el.getAttribute("data-df-kind");
    if (k) return k;
    return el.tagName ? el.tagName.toLowerCase() : "element";
  }

  function elementLabel(el) {
    const kind = elementKind(el);
    switch (kind) {
      case "text": return "Text";
      case "rect": return "Rect";
      case "ellipse": return "Circle";
      case "line": return "Line";
      case "arrow": return "Arrow";
      case "image": return "Image";
      case "table": return "Table";
      case "td": return "Cell";
      case "th": return "Header";
      default: return el.tagName ? el.tagName.toLowerCase() : "el";
    }
  }

  function isTableCell(el) {
    return !!(el && (el.tagName === "TD" || el.tagName === "TH"));
  }

  function isTextEditable(el) {
    if (!el) return false;
    const kind = elementKind(el);
    if (kind === "image" || kind === "line" || kind === "arrow" || el.tagName === "TABLE") return false;
    if (VOID_TAGS.has(el.tagName)) return false;
    return true;
  }

  function isTransparentBg(bg) {
    if (!bg || bg === "transparent") return true;
    const m = bg.match(/rgba\(\s*\d+\s*,\s*\d+\s*,\s*\d+\s*,\s*([\d.]+)\s*\)/i);
    return m ? parseFloat(m[1]) === 0 : false;
  }

  function cleanCloneHtml(doc) {
    if (!doc || !doc.documentElement) return "";
    const clone = doc.documentElement.cloneNode(true);
    clone.querySelectorAll("[data-df-editor-ui='1']").forEach((n) => n.remove());
    clone.querySelectorAll("[contenteditable]").forEach((n) => n.removeAttribute("contenteditable"));
    clone.querySelectorAll("[data-df-editing]").forEach((n) => n.removeAttribute("data-df-editing"));
    clone.querySelectorAll("[data-df-selected]").forEach((n) => n.removeAttribute("data-df-selected"));
    return "<!DOCTYPE html>\n" + clone.outerHTML;
  }

  function createController() {
    let channel = null;
    let frame = null;
    let doc = null;
    let selected = null;
    let editing = null;
    let uiLayer = null;
    let hoverBox = null;
    let selectBox = null;
    let badgeEl = null;
    let moveHandleEl = null;
    let handleEls = {};
    let dragState = null;
    let history = [];
    let historyIdx = -1;
    let inputTimer = null;
    let nextZ = 100;

    function send(evt) {
      if (channel) {
        try {
          channel.send(evt);
        } catch (_) {}
      }
    }

    function canUndo() {
      return historyIdx > 0;
    }

    function canRedo() {
      return historyIdx >= 0 && historyIdx < history.length - 1;
    }

    function pushHistory(html) {
      if (historyIdx >= 0 && history[historyIdx] === html) return;
      history = history.slice(0, historyIdx + 1);
      history.push(html);
      if (history.length > MAX_HISTORY) {
        history.shift();
      }
      historyIdx = history.length - 1;
    }

    function emitHtml(recordHistory) {
      if (!doc) return;
      const html = cleanCloneHtml(doc);
      if (recordHistory) pushHistory(html);
      send({
        type: "html_changed",
        html,
        can_undo: canUndo(),
        can_redo: canRedo(),
      });
    }

    function readSelectionInfo(el) {
      if (!el || !doc || !doc.defaultView) return null;
      const win = doc.defaultView;
      const cs = win.getComputedStyle(el);
      const rect = el.getBoundingClientRect();
      const bodyRect = doc.body.getBoundingClientRect();
      const bgHex = isTransparentBg(cs.backgroundColor) ? "transparent" : rgbToHex(cs.backgroundColor);
      const borderHex = rgbToHex(cs.borderTopColor);
      const borderWidth = Math.round(parseFloat(cs.borderTopWidth) || 0);
      const radius = Math.round(parseFloat(cs.borderTopLeftRadius) || 0);
      const fontSize = Math.max(8, Math.round(parseFloat(cs.fontSize) || 14));
      const opacity = Math.round((parseFloat(cs.opacity) || 1) * 100);
      const weight = cs.fontWeight;
      const bold = weight === "bold" || (parseInt(weight, 10) || 400) >= 600;
      const italic = cs.fontStyle === "italic";
      const underline = (cs.textDecorationLine || cs.textDecoration || "").includes("underline");
      const textAlign = ["center", "right"].includes(cs.textAlign) ? cs.textAlign : "left";

      let inTable = false;
      let tableRows = 0;
      let tableCols = 0;
      let cellRow = 0;
      let cellCol = 0;
      let isHeaderCell = false;

      const tableEl = el.tagName === "TABLE" ? el : (el.closest ? el.closest("table") : null);
      if (tableEl && tableEl.rows) {
        inTable = true;
        tableRows = tableEl.rows.length;
        for (let r = 0; r < tableEl.rows.length; r++) {
          tableCols = Math.max(tableCols, tableEl.rows[r].cells.length);
        }
        if (isTableCell(el)) {
          isHeaderCell = el.tagName === "TH";
          const rowEl = el.parentElement;
          if (rowEl && typeof rowEl.rowIndex === "number") {
            cellRow = rowEl.rowIndex + 1;
          }
          if (typeof el.cellIndex === "number") {
            cellCol = el.cellIndex + 1;
          }
        }
      }

      return {
        label: elementLabel(el),
        kind: elementKind(el),
        x: Math.round(rect.left - bodyRect.left),
        y: Math.round(rect.top - bodyRect.top),
        width: Math.max(1, Math.round(rect.width)),
        height: Math.max(1, Math.round(rect.height)),
        font_size: fontSize,
        color: rgbToHex(cs.color),
        background: bgHex,
        border_color: borderHex,
        border_width: borderWidth,
        border_radius: radius,
        opacity,
        bold,
        italic,
        underline,
        text_align: textAlign,
        is_text_editable: isTextEditable(el),
        in_table: inTable,
        table_rows: tableRows,
        table_cols: tableCols,
        cell_row: cellRow,
        cell_col: cellCol,
        is_header_cell: isHeaderCell,
      };
    }

    function emitSelection() {
      send({
        type: "selection_changed",
        info: readSelectionInfo(selected),
        can_undo: canUndo(),
        can_redo: canRedo(),
      });
    }

    function updateHoverOverlay(el) {
      if (!hoverBox || !uiLayer || !doc) return;
      if (!el || el === selected || editing || dragState || !doc.body.contains(el)) {
        hoverBox.style.display = "none";
        return;
      }
      const uiRect = uiLayer.getBoundingClientRect();
      const rect = el.getBoundingClientRect();
      hoverBox.style.display = "block";
      hoverBox.style.left = rect.left - uiRect.left + "px";
      hoverBox.style.top = rect.top - uiRect.top + "px";
      hoverBox.style.width = Math.max(4, rect.width) + "px";
      hoverBox.style.height = Math.max(4, rect.height) + "px";
    }

    function updateOverlay() {
      if (!selectBox || !uiLayer || !doc) return;
      if (!selected || !doc.body.contains(selected)) {
        selected = null;
        selectBox.style.display = "none";
        return;
      }
      const uiRect = uiLayer.getBoundingClientRect();
      const rect = selected.getBoundingClientRect();
      const relLeft = rect.left - uiRect.left;
      const relTop = rect.top - uiRect.top;
      selectBox.style.display = "block";
      selectBox.style.left = relLeft + "px";
      selectBox.style.top = relTop + "px";
      selectBox.style.width = Math.max(4, rect.width) + "px";
      selectBox.style.height = Math.max(4, rect.height) + "px";
      selectBox.classList.toggle("is-editing", editing === selected);
      selectBox.classList.toggle("is-top-edge", relTop < 24);
      if (editing || dragState) {
        updateHoverOverlay(null);
      }

      if (moveHandleEl) {
        if (editing === selected || dragState) {
          moveHandleEl.style.display = "none";
        } else {
          const isTbl = selected.tagName === "TABLE" || isTableCell(selected);
          moveHandleEl.textContent = isTbl ? "⠿ Move Table" : "⠿ Move";
          moveHandleEl.style.display = "inline-flex";
        }
      }

      if (badgeEl) {
        if (dragState && dragState.mode === "move") {
          badgeEl.textContent = `${Math.round(relLeft)}, ${Math.round(relTop)}`;
          badgeEl.style.display = "block";
        } else if (dragState && dragState.mode === "resize") {
          badgeEl.textContent = `${Math.round(rect.width)} × ${Math.round(rect.height)}`;
          badgeEl.style.display = "block";
        } else {
          badgeEl.style.display = "none";
        }
      }
    }

    function ensureOverlay() {
      if (!doc || !doc.head || !doc.body) return;
      if (!doc.getElementById(STYLE_ID)) {
        const st = doc.createElement("style");
        st.id = STYLE_ID;
        st.setAttribute("data-df-editor-ui", "1");
        st.textContent = `
          html { min-height: 100%; }
          body { min-height: 100vh; position: relative; }
          #${UI_ID} {
            position: absolute; inset: 0; pointer-events: none; z-index: 2147483640;
          }
          #${UI_ID} .df-hover-box {
            position: absolute; display: none; box-sizing: border-box;
            border: 1px dashed rgba(22, 163, 74, 0.65);
            background: rgba(22, 163, 74, 0.05); pointer-events: none;
          }
          #${UI_ID} .df-select-box {
            position: absolute; display: none; box-sizing: border-box;
            border: 1.5px solid #16a34a; pointer-events: none;
          }
          #${UI_ID} .df-select-box.is-editing {
            border-color: #22c55e; border-style: dashed;
          }
          #${UI_ID} .df-handle {
            position: absolute; width: 9px; height: 9px; background: #ffffff;
            border: 1.5px solid #16a34a; border-radius: 2px;
            pointer-events: auto; box-sizing: border-box;
          }
          #${UI_ID} .df-select-box.is-editing .df-handle { display: none; }
          #${UI_ID} .df-handle[data-h="nw"] { left: -5px; top: -5px; cursor: nwse-resize; }
          #${UI_ID} .df-handle[data-h="n"]  { left: calc(50% - 4px); top: -5px; cursor: ns-resize; }
          #${UI_ID} .df-handle[data-h="ne"] { right: -5px; top: -5px; cursor: nesw-resize; }
          #${UI_ID} .df-handle[data-h="e"]  { right: -5px; top: calc(50% - 4px); cursor: ew-resize; }
          #${UI_ID} .df-handle[data-h="se"] { right: -5px; bottom: -5px; cursor: nwse-resize; }
          #${UI_ID} .df-handle[data-h="s"]  { left: calc(50% - 4px); bottom: -5px; cursor: ns-resize; }
          #${UI_ID} .df-handle[data-h="sw"] { left: -5px; bottom: -5px; cursor: nesw-resize; }
          #${UI_ID} .df-handle[data-h="w"]  { left: -5px; top: calc(50% - 4px); cursor: ew-resize; }
          #${UI_ID} .df-move-handle {
            position: absolute; left: 0; top: -22px; display: none;
            align-items: center; gap: 4px; height: 18px; padding: 0 7px;
            border-radius: 4px; font: 600 10px/18px system-ui, sans-serif;
            background: #16a34a; color: #050706; white-space: nowrap;
            pointer-events: auto; cursor: grab; user-select: none;
            box-shadow: 0 2px 6px rgba(0,0,0,.35);
          }
          #${UI_ID} .df-move-handle:active { cursor: grabbing; }
          #${UI_ID} .df-select-box.is-top-edge .df-move-handle { top: 2px; left: 2px; }
          #${UI_ID} .df-select-box.is-editing .df-move-handle { display: none !important; }
          #${UI_ID} .df-badge {
            position: absolute; left: 0; top: -22px; display: none;
            padding: 1px 6px; border-radius: 4px; font: 600 11px/16px system-ui, sans-serif;
            background: #111827; color: #f8fafc; white-space: nowrap;
            box-shadow: 0 2px 6px rgba(0,0,0,.35);
          }
          #${UI_ID} .df-select-box.is-top-edge .df-badge { top: 2px; left: 2px; }
          table[data-df-selected="1"], table[data-df-selected="1"] td, table[data-df-selected="1"] th {
            cursor: move !important;
          }
          [data-df-editing="1"] { outline: none !important; cursor: text !important; }
        `;
        doc.head.appendChild(st);
      }

      if (!doc.getElementById(UI_ID)) {
        uiLayer = doc.createElement("div");
        uiLayer.id = UI_ID;
        uiLayer.setAttribute("data-df-editor-ui", "1");

        selectBox = doc.createElement("div");
        selectBox.className = "df-select-box";
        selectBox.setAttribute("data-df-editor-ui", "1");

        moveHandleEl = doc.createElement("div");
        moveHandleEl.className = "df-move-handle";
        moveHandleEl.textContent = "⠿ Move";
        moveHandleEl.setAttribute("data-df-editor-ui", "1");
        moveHandleEl.addEventListener("mousedown", (e) => {
          if (e.button !== 0 || !selected || !doc) return;
          e.preventDefault();
          e.stopPropagation();
          let target = selected;
          if (isTableCell(selected)) {
            const tbl = selected.closest ? selected.closest("table") : null;
            if (tbl) {
              selectElement(tbl);
              target = tbl;
            }
          }
          startMove(e, target);
        });
        selectBox.appendChild(moveHandleEl);

        badgeEl = doc.createElement("div");
        badgeEl.className = "df-badge";
        badgeEl.setAttribute("data-df-editor-ui", "1");
        selectBox.appendChild(badgeEl);

        handleEls = {};
        for (const h of HANDLES) {
          const hd = doc.createElement("div");
          hd.className = "df-handle";
          hd.setAttribute("data-h", h);
          hd.setAttribute("data-df-editor-ui", "1");
          hd.addEventListener("mousedown", (e) => startResize(e, h));
          selectBox.appendChild(hd);
          handleEls[h] = hd;
        }

        hoverBox = doc.createElement("div");
        hoverBox.className = "df-hover-box";
        hoverBox.setAttribute("data-df-editor-ui", "1");
        uiLayer.appendChild(hoverBox);

        uiLayer.appendChild(selectBox);
        doc.body.appendChild(uiLayer);
      } else {
        uiLayer = doc.getElementById(UI_ID);
        hoverBox = uiLayer.querySelector(".df-hover-box");
        selectBox = uiLayer.querySelector(".df-select-box");
        moveHandleEl = uiLayer.querySelector(".df-move-handle");
        badgeEl = uiLayer.querySelector(".df-badge");
      }
    }

    function stopEditing(commit) {
      if (!editing) return;
      const el = editing;
      editing = null;
      el.removeAttribute("contenteditable");
      el.removeAttribute("data-df-editing");
      if (inputTimer) {
        clearTimeout(inputTimer);
        inputTimer = null;
      }
      updateOverlay();
      if (commit) {
        emitHtml(true);
      }
      emitSelection();
    }

    function selectElement(el) {
      if (el && (el === doc.body || el === doc.documentElement || isEditorNode(el))) {
        el = null;
      }
      if (editing && editing !== el) {
        stopEditing(true);
      }
      if (selected && selected !== el && selected.removeAttribute) {
        selected.removeAttribute("data-df-selected");
      }
      selected = el;
      if (selected && selected.tagName === "TABLE") {
        selected.setAttribute("data-df-selected", "1");
      }
      updateOverlay();
      emitSelection();
    }

    function startEditing(el) {
      if (!el || !isTextEditable(el)) return;
      if (editing === el) return;
      selectElement(el);
      editing = el;
      el.setAttribute("contenteditable", "true");
      el.setAttribute("data-df-editing", "1");
      el.focus();
      updateOverlay();
    }

    const INLINE_TAGS = new Set([
      "SPAN", "STRONG", "B", "EM", "I", "U", "SMALL", "MARK", "SUB", "SUP", "A", "CODE",
    ]);
    const TEXT_BLOCK_TAGS = new Set([
      "P", "H1", "H2", "H3", "H4", "H5", "H6", "LI", "DT", "DD", "BLOCKQUOTE", "FIGCAPTION", "LABEL",
    ]);
    const TABLE_STRUCT_TAGS = new Set(["TR", "THEAD", "TBODY", "TFOOT"]);

    function hasDirectText(el) {
      for (const n of el.childNodes) {
        if (n.nodeType === 3 && n.textContent && n.textContent.trim().length > 0) return true;
      }
      return false;
    }

    function isPageWrapper(el) {
      if (!el || !doc || !doc.body) return false;
      if (el === doc.body || el === doc.documentElement) return true;
      if (el.getAttribute && el.getAttribute("data-df-placed") === "1") return false;
      if (el.classList && el.classList.contains("pdf-page")) return true;
      if (!["SECTION", "MAIN", "ARTICLE", "DIV"].includes(el.tagName)) return false;
      if (el.children.length < 2 || hasDirectText(el)) return false;
      const er = el.getBoundingClientRect();
      const br = doc.body.getBoundingClientRect();
      return (
        br.width > 0 &&
        br.height > 0 &&
        er.width >= br.width * 0.65 &&
        er.height >= br.height * 0.65
      );
    }

    function resolveSelectableTarget(target, clientX, clientY) {
      if (!target || !doc) return null;
      if (target.nodeType !== 1) target = target.parentElement;
      if (!target || isEditorNode(target)) return null;
      if (target === doc.body || target === doc.documentElement) return null;
      if (target.closest) {
        const cell = target.closest("td, th");
        if (cell) return cell;
      }
      if (TABLE_STRUCT_TAGS.has(target.tagName) && target.closest) {
        const tbl = target.closest("table");
        if (tbl) return tbl;
      }
      let cur = target;
      while (cur && cur !== doc.body && cur !== doc.documentElement) {
        if (cur.getAttribute && cur.getAttribute("data-df-placed") === "1") {
          return cur;
        }
        cur = cur.parentElement;
      }
      if (target.closest) {
        const placed = target.closest("[data-df-placed='1']");
        if (placed) return placed;
        if (target.tagName === "svg" || target.closest("svg")) {
          const svg = target.tagName === "svg" ? target : target.closest("svg");
          if (svg && svg.parentElement && svg.parentElement.getAttribute("data-df-placed") === "1") {
            return svg.parentElement;
          }
          return svg;
        }
      }
      if (INLINE_TAGS.has(target.tagName) && doc.defaultView) {
        const disp = doc.defaultView.getComputedStyle(target).display;
        if (
          disp === "inline" &&
          target.parentElement &&
          TEXT_BLOCK_TAGS.has(target.parentElement.tagName)
        ) {
          target = target.parentElement;
        }
      }
      if (isPageWrapper(target)) {
        if (typeof clientX === "number" && typeof clientY === "number" && doc.elementsFromPoint) {
          const stack = doc.elementsFromPoint(clientX, clientY);
          for (const cand of stack) {
            if (
              cand &&
              cand !== target &&
              !isEditorNode(cand) &&
              cand !== doc.body &&
              cand !== doc.documentElement &&
              !isPageWrapper(cand)
            ) {
              return resolveSelectableTarget(cand);
            }
          }
        }
        return null;
      }
      return target;
    }

    function startMove(e, el, pendingCell) {
      if (isTableCell(el)) return;
      e.preventDefault();
      dragState = {
        mode: "move",
        el,
        pendingCell: pendingCell || null,
        startX: e.clientX,
        startY: e.clientY,
        startLeft: 0,
        startTop: 0,
        initialized: false,
        moved: false,
      };
    }

    function startResize(e, handle) {
      if (!selected || !doc) return;
      e.preventDefault();
      e.stopPropagation();
      const el = selected;
      const win = doc.defaultView;
      const cs = win.getComputedStyle(el);
      const rect = el.getBoundingClientRect();
      let pos = cs.position;
      if (pos === "static") {
        pos = el.getAttribute("data-df-placed") === "1" ? "absolute" : "relative";
        el.style.position = pos;
      }
      const startLeft = parseFloat(el.style.left) || (pos === "relative" ? 0 : el.offsetLeft);
      const startTop = parseFloat(el.style.top) || (pos === "relative" ? 0 : el.offsetTop);

      dragState = {
        mode: "resize",
        handle,
        el,
        startX: e.clientX,
        startY: e.clientY,
        startLeft,
        startTop,
        startW: rect.width,
        startH: rect.height,
        moved: false,
      };
    }

    function onMouseMove(e) {
      if (!dragState) {
        if (!editing && e.target) {
          updateHoverOverlay(resolveSelectableTarget(e.target, e.clientX, e.clientY));
        }
        return;
      }
      const dx = e.clientX - dragState.startX;
      const dy = e.clientY - dragState.startY;
      if (!dragState.moved && Math.abs(dx) < 3 && Math.abs(dy) < 3) return;
      dragState.moved = true;
      updateHoverOverlay(null);

      const free = e.altKey;
      const el = dragState.el;
      if (dragState.mode === "move") {
        if (!dragState.initialized) {
          const win = doc.defaultView;
          const cs = win.getComputedStyle(el);
          const isPlaced = el.getAttribute("data-df-placed") === "1";
          let pos = cs.position;
          if (pos === "static") {
            pos = isPlaced ? "absolute" : "relative";
            el.style.position = pos;
          }
          dragState.startLeft = parseFloat(el.style.left) || (pos === "relative" ? 0 : el.offsetLeft);
          dragState.startTop = parseFloat(el.style.top) || (pos === "relative" ? 0 : el.offsetTop);
          if (doc.body) {
            doc.body.style.cursor = "grabbing";
          }
          dragState.initialized = true;
        }
        el.style.left = snap(dragState.startLeft + dx, free) + "px";
        el.style.top = snap(dragState.startTop + dy, free) + "px";
      } else if (dragState.mode === "resize") {
        const h = dragState.handle;
        let newW = dragState.startW;
        let newH = dragState.startH;
        let newL = dragState.startLeft;
        let newT = dragState.startTop;

        if (h.includes("e")) newW = Math.max(12, dragState.startW + dx);
        if (h.includes("s")) newH = Math.max(4, dragState.startH + dy);
        if (h.includes("w")) {
          const w = Math.max(12, dragState.startW - dx);
          newL = dragState.startLeft + (dragState.startW - w);
          newW = w;
        }
        if (h.includes("n")) {
          const ht = Math.max(4, dragState.startH - dy);
          newT = dragState.startTop + (dragState.startH - ht);
          newH = ht;
        }

        if (e.shiftKey && h.length === 2 && dragState.startH > 0) {
          const ratio = dragState.startW / dragState.startH;
          newH = Math.max(4, Math.round(newW / ratio));
        }

        el.style.width = snap(newW, free) + "px";
        if (elementKind(el) !== "line") {
          el.style.height = snap(newH, free) + "px";
        }
        if (h.includes("w")) el.style.left = snap(newL, free) + "px";
        if (h.includes("n")) el.style.top = snap(newT, free) + "px";
      }
      updateOverlay();
    }

    function onMouseUp() {
      if (!dragState) return;
      const { moved, pendingCell } = dragState;
      dragState = null;
      if (doc && doc.body) {
        doc.body.style.cursor = "";
      }
      if (!moved && pendingCell) {
        selectElement(pendingCell);
        return;
      }
      updateOverlay();
      if (moved) {
        emitHtml(true);
        emitSelection();
      }
    }

    function onKeyDown(e) {
      if (!doc) return;
      if (editing) {
        if (e.key === "Escape") {
          e.preventDefault();
          stopEditing(true);
        } else if (e.key === "Tab" && isTableCell(editing)) {
          e.preventDefault();
          const table = editing.closest("table");
          if (table) {
            const cells = Array.from(table.querySelectorAll("th, td"));
            const idx = cells.indexOf(editing);
            const nextIdx = e.shiftKey ? idx - 1 : idx + 1;
            stopEditing(true);
            if (nextIdx >= 0 && nextIdx < cells.length) {
              startEditing(cells[nextIdx]);
            }
          }
        }
        return;
      }

      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "z") {
        e.preventDefault();
        if (e.shiftKey) handleCommand({ type: "redo" });
        else handleCommand({ type: "undo" });
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "y") {
        e.preventDefault();
        handleCommand({ type: "redo" });
        return;
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "d" && selected) {
        e.preventDefault();
        handleCommand({ type: "duplicate_selected" });
        return;
      }

      if (!selected) return;
      if (e.key === "Escape") {
        selectElement(null);
      } else if ((e.key === "Enter" || e.key === "F2") && isTableCell(selected)) {
        e.preventDefault();
        startEditing(selected);
      } else if (e.key === "Tab" && isTableCell(selected)) {
        e.preventDefault();
        const table = selected.closest("table");
        if (table) {
          const cells = Array.from(table.querySelectorAll("th, td"));
          const idx = cells.indexOf(selected);
          const nextIdx = e.shiftKey ? idx - 1 : idx + 1;
          if (nextIdx >= 0 && nextIdx < cells.length) {
            selectElement(cells[nextIdx]);
          }
        }
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        handleCommand({ type: "delete_selected" });
      } else if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
        e.preventDefault();
        if (isTableCell(selected)) {
          const table = selected.closest("table");
          const rowEl = selected.parentElement;
          if (table && rowEl && typeof rowEl.rowIndex === "number" && typeof selected.cellIndex === "number") {
            let r = rowEl.rowIndex;
            let c = selected.cellIndex;
            if (e.key === "ArrowLeft") c -= 1;
            if (e.key === "ArrowRight") c += 1;
            if (e.key === "ArrowUp") r -= 1;
            if (e.key === "ArrowDown") r += 1;
            if (r >= 0 && r < table.rows.length) {
              const targetRow = table.rows[r];
              if (c >= 0 && c < targetRow.cells.length) {
                selectElement(targetRow.cells[c]);
              }
            }
          }
          return;
        }
        const step = e.shiftKey ? 10 : 1;
        const win = doc.defaultView;
        const cs = win.getComputedStyle(selected);
        if (cs.position === "static") {
          selected.style.position = selected.getAttribute("data-df-placed") === "1" ? "absolute" : "relative";
        }
        let l = parseFloat(selected.style.left) || (selected.style.position === "relative" ? 0 : selected.offsetLeft);
        let t = parseFloat(selected.style.top) || (selected.style.position === "relative" ? 0 : selected.offsetTop);
        if (e.key === "ArrowLeft") l -= step;
        if (e.key === "ArrowRight") l += step;
        if (e.key === "ArrowUp") t -= step;
        if (e.key === "ArrowDown") t += step;
        selected.style.left = l + "px";
        selected.style.top = t + "px";
        updateOverlay();
        emitHtml(true);
        emitSelection();
      }
    }

    function bindIframeEvents() {
      if (!doc || !doc.defaultView) return;
      doc.addEventListener("mousedown", (e) => {
        if (e.button !== 0) return;
        if (isEditorNode(e.target)) return;
        if (selected && selected.tagName === "TABLE" && selected.contains(e.target)) {
          const clickedCell = e.target.closest ? e.target.closest("td, th") : null;
          startMove(e, selected, clickedCell);
          return;
        }
        const target = resolveSelectableTarget(e.target, e.clientX, e.clientY);
        if (!target) {
          selectElement(null);
          return;
        }
        if (editing === target) return;
        selectElement(target);
        startMove(e, target);
      });

      doc.addEventListener("dblclick", (e) => {
        if (isEditorNode(e.target)) return;
        const target = resolveSelectableTarget(e.target, e.clientX, e.clientY);
        if (target && isTextEditable(target)) {
          e.preventDefault();
          startEditing(target);
        }
      });

      doc.addEventListener("mouseleave", () => {
        updateHoverOverlay(null);
      });

      doc.addEventListener("input", () => {
        if (!editing) return;
        updateOverlay();
        if (inputTimer) clearTimeout(inputTimer);
        inputTimer = setTimeout(() => {
          emitHtml(true);
        }, 120);
      });

      doc.addEventListener("blur", (e) => {
        if (editing && e.target === editing) {
          stopEditing(true);
        }
      }, true);

      doc.defaultView.addEventListener("blur", () => {
        if (editing) {
          stopEditing(true);
        }
      });

      doc.addEventListener("mousemove", onMouseMove);
      doc.addEventListener("mouseup", onMouseUp);
      doc.addEventListener("keydown", onKeyDown);
      doc.defaultView.addEventListener("scroll", updateOverlay);
      doc.defaultView.addEventListener("resize", updateOverlay);
    }

    function loadHtml(html, resetHistory) {
      frame = document.getElementById(FRAME_ID);
      if (!frame) return;
      doc = frame.contentDocument || (frame.contentWindow && frame.contentWindow.document);
      if (!doc) return;

      selected = null;
      editing = null;
      dragState = null;

      doc.open();
      doc.write(html);
      doc.close();

      ensureOverlay();
      bindIframeEvents();

      const clean = cleanCloneHtml(doc);
      if (resetHistory) {
        history = [clean];
        historyIdx = 0;
      }
      updateOverlay();
      emitSelection();
    }

    function placeNewElement(el, startEdit) {
      if (!doc || !doc.body) return;
      const scrollY = doc.defaultView ? doc.defaultView.scrollY : 0;
      el.setAttribute("data-df-placed", "1");
      if (!el.style.position) el.style.position = "absolute";
      if (!el.style.left) el.style.left = "48px";
      if (!el.style.top) el.style.top = Math.round(48 + scrollY) + "px";
      el.style.zIndex = String(++nextZ);
      if (uiLayer && uiLayer.parentNode === doc.body) {
        doc.body.insertBefore(el, uiLayer);
      } else {
        doc.body.appendChild(el);
      }
      selectElement(el);
      emitHtml(true);
      if (startEdit) {
        startEditing(el);
      }
    }

    function insertText() {
      const el = doc.createElement("div");
      el.setAttribute("data-df-kind", "text");
      el.style.width = "220px";
      el.style.minHeight = "32px";
      el.style.padding = "6px 8px";
      el.style.fontSize = "16px";
      el.style.color = "#111827";
      el.style.background = "transparent";
      el.style.lineHeight = "1.4";
      el.textContent = "Double-click to edit text";
      placeNewElement(el, true);
    }

    function insertShape(shape) {
      if (shape === "rect") {
        const el = doc.createElement("div");
        el.setAttribute("data-df-kind", "rect");
        el.style.width = "160px";
        el.style.height = "90px";
        el.style.background = "#dcfce7";
        el.style.border = "2px solid #16a34a";
        el.style.borderRadius = "6px";
        el.style.display = "flex";
        el.style.alignItems = "center";
        el.style.justifyContent = "center";
        el.style.color = "#1e293b";
        el.style.fontSize = "14px";
        placeNewElement(el, false);
      } else if (shape === "ellipse") {
        const el = doc.createElement("div");
        el.setAttribute("data-df-kind", "ellipse");
        el.style.width = "110px";
        el.style.height = "110px";
        el.style.background = "#bbf7d0";
        el.style.border = "2px solid #15803d";
        el.style.borderRadius = "50%";
        el.style.display = "flex";
        el.style.alignItems = "center";
        el.style.justifyContent = "center";
        el.style.color = "#1e293b";
        placeNewElement(el, false);
      } else if (shape === "line") {
        const el = doc.createElement("div");
        el.setAttribute("data-df-kind", "line");
        el.style.width = "260px";
        el.style.height = "2px";
        el.style.background = "#1e293b";
        placeNewElement(el, false);
      } else if (shape === "arrow") {
        const el = doc.createElement("div");
        el.setAttribute("data-df-kind", "arrow");
        el.style.width = "160px";
        el.style.height = "36px";
        el.style.color = "#1e293b";
        el.innerHTML =
          '<svg viewBox="0 0 160 36" width="100%" height="100%" preserveAspectRatio="none" style="display:block;pointer-events:none;"><line x1="4" y1="18" x2="142" y2="18" stroke="currentColor" stroke-width="3"/><polygon points="140,8 156,18 140,28" fill="currentColor"/></svg>';
        placeNewElement(el, false);
      }
    }

    function insertImage(dataUrl, alt) {
      const wrap = doc.createElement("div");
      wrap.setAttribute("data-df-kind", "image");
      wrap.style.width = "180px";
      wrap.style.height = "135px";
      wrap.style.overflow = "hidden";
      const img = doc.createElement("img");
      img.src = dataUrl;
      img.alt = alt || "Inserted image";
      img.draggable = false;
      img.style.width = "100%";
      img.style.height = "100%";
      img.style.objectFit = "contain";
      img.style.display = "block";
      img.style.pointerEvents = "none";
      wrap.appendChild(img);
      placeNewElement(wrap, false);
    }

    function makeTableCell(tag, text) {
      const cell = doc.createElement(tag);
      cell.style.border = "1px solid #cbd5e1";
      cell.style.padding = "7px 10px";
      cell.style.textAlign = "left";
      cell.style.minWidth = "60px";
      if (tag === "th") {
        cell.style.background = "#f1f5f9";
        cell.style.fontWeight = "600";
      }
      cell.textContent = text;
      return cell;
    }

    function insertTable(rows, cols) {
      const rCount = Math.max(1, Math.min(20, parseInt(rows, 10) || 3));
      const cCount = Math.max(1, Math.min(12, parseInt(cols, 10) || 3));
      const table = doc.createElement("table");
      table.setAttribute("data-df-kind", "table");
      table.style.borderCollapse = "collapse";
      table.style.width = "480px";
      table.style.background = "#ffffff";
      table.style.color = "#111827";
      table.style.fontSize = "13px";

      const thead = doc.createElement("thead");
      const hRow = doc.createElement("tr");
      for (let c = 0; c < cCount; c++) {
        hRow.appendChild(makeTableCell("th", `Column ${c + 1}`));
      }
      thead.appendChild(hRow);
      table.appendChild(thead);

      if (rCount > 1) {
        const tbody = doc.createElement("tbody");
        for (let r = 1; r < rCount; r++) {
          const tr = doc.createElement("tr");
          for (let c = 0; c < cCount; c++) {
            tr.appendChild(makeTableCell("td", `Cell ${r}.${c + 1}`));
          }
          tbody.appendChild(tr);
        }
        table.appendChild(tbody);
      }

      placeNewElement(table, false);
    }

    function applyTableAction(action) {
      if (!selected || !doc) return;
      const cell = isTableCell(selected) ? selected : null;
      const table = selected.tagName === "TABLE" ? selected : (selected.closest ? selected.closest("table") : null);
      if (!table || !table.rows || table.rows.length === 0) return;

      if (action === "select_table") {
        selectElement(table);
        return;
      }

      const row = cell ? cell.parentElement : table.rows[table.rows.length - 1];
      const rowIdx = row && typeof row.rowIndex === "number" ? row.rowIndex : table.rows.length - 1;
      const colIdx = cell && typeof cell.cellIndex === "number" ? cell.cellIndex : 0;
      const maxCols = Array.from(table.rows).reduce((m, r) => Math.max(m, r.cells.length), 1);

      let nextSelect = selected;

      switch (action) {
        case "insert_row_above":
        case "insert_row_below": {
          const container = row && row.parentElement ? row.parentElement : table;
          const inThead = container.tagName === "THEAD";
          const cellTag = inThead && action === "insert_row_above" ? "th" : "td";
          const newTr = doc.createElement("tr");
          for (let c = 0; c < maxCols; c++) {
            newTr.appendChild(makeTableCell(cellTag, ""));
          }
          if (inThead && action === "insert_row_below") {
            const tbody = table.tBodies[0] || table.appendChild(doc.createElement("tbody"));
            tbody.insertBefore(newTr, tbody.firstChild);
          } else if (action === "insert_row_above") {
            container.insertBefore(newTr, row);
          } else {
            container.insertBefore(newTr, row ? row.nextSibling : null);
          }
          nextSelect = newTr.cells[Math.min(colIdx, newTr.cells.length - 1)] || table;
          break;
        }
        case "delete_row": {
          if (table.rows.length <= 1) return;
          const targetIdx = Math.min(rowIdx, table.rows.length - 1);
          const victim = table.rows[targetIdx];
          if (victim && victim.parentNode) {
            victim.remove();
          }
          const fallbackRow = table.rows[Math.min(targetIdx, table.rows.length - 1)];
          nextSelect = fallbackRow
            ? (fallbackRow.cells[Math.min(colIdx, fallbackRow.cells.length - 1)] || table)
            : table;
          break;
        }
        case "insert_col_left":
        case "insert_col_right": {
          const insertAt = action === "insert_col_left" ? colIdx : colIdx + 1;
          for (let r = 0; r < table.rows.length; r++) {
            const tr = table.rows[r];
            const isHdr =
              (tr.parentElement && tr.parentElement.tagName === "THEAD") ||
              (tr.cells.length > 0 && Array.from(tr.cells).every((c) => c.tagName === "TH"));
            const newCell = makeTableCell(isHdr ? "th" : "td", isHdr ? "Header" : "");
            if (insertAt >= tr.cells.length) {
              tr.appendChild(newCell);
            } else {
              tr.insertBefore(newCell, tr.cells[insertAt]);
            }
            if (r === rowIdx) {
              nextSelect = newCell;
            }
          }
          break;
        }
        case "delete_col": {
          if (maxCols <= 1) return;
          for (let r = 0; r < table.rows.length; r++) {
            const tr = table.rows[r];
            if (tr.cells.length > 1 && colIdx < tr.cells.length) {
              tr.deleteCell(colIdx);
            }
          }
          const curRow = table.rows[Math.min(rowIdx, table.rows.length - 1)];
          nextSelect = curRow
            ? (curRow.cells[Math.min(colIdx, curRow.cells.length - 1)] || table)
            : table;
          break;
        }
        case "toggle_header": {
          if (cell) {
            const newTag = cell.tagName === "TH" ? "td" : "th";
            const replacement = doc.createElement(newTag);
            replacement.innerHTML = cell.innerHTML;
            for (const attr of cell.attributes) {
              replacement.setAttribute(attr.name, attr.value);
            }
            if (newTag === "th") {
              replacement.style.background = "#f1f5f9";
              replacement.style.fontWeight = "600";
            } else {
              replacement.style.background = "";
              replacement.style.fontWeight = "";
            }
            cell.parentNode.replaceChild(replacement, cell);
            nextSelect = replacement;
          }
          break;
        }
      }

      selectElement(nextSelect);
      emitHtml(true);
    }

    function applyStyle(prop, value) {
      if (!selected) return;
      switch (prop) {
        case "font_size":
          selected.style.fontSize = value + "px";
          break;
        case "color":
          selected.style.color = value;
          break;
        case "background":
          selected.style.background = value;
          break;
        case "border_color":
          selected.style.borderColor = value;
          if ((parseFloat(doc.defaultView.getComputedStyle(selected).borderTopWidth) || 0) === 0) {
            selected.style.borderStyle = "solid";
            selected.style.borderWidth = "1px";
          }
          break;
        case "border_width": {
          const w = Math.max(0, parseInt(value, 10) || 0);
          selected.style.borderStyle = w > 0 ? "solid" : "none";
          selected.style.borderWidth = w + "px";
          break;
        }
        case "border_radius":
          selected.style.borderRadius = value + "px";
          break;
        case "opacity":
          selected.style.opacity = String(Math.max(0, Math.min(100, parseInt(value, 10) || 100)) / 100);
          break;
      }
      updateOverlay();
      emitHtml(true);
      emitSelection();
    }

    function applyFormat(cmd) {
      if (!selected) return;
      const cs = doc.defaultView.getComputedStyle(selected);
      switch (cmd) {
        case "bold": {
          const isBold = cs.fontWeight === "bold" || (parseInt(cs.fontWeight, 10) || 400) >= 600;
          selected.style.fontWeight = isBold ? "400" : "700";
          break;
        }
        case "italic":
          selected.style.fontStyle = cs.fontStyle === "italic" ? "normal" : "italic";
          break;
        case "underline": {
          const hasU = (cs.textDecorationLine || cs.textDecoration || "").includes("underline");
          selected.style.textDecoration = hasU ? "none" : "underline";
          break;
        }
        case "align_left":
          selected.style.textAlign = "left";
          break;
        case "align_center":
          selected.style.textAlign = "center";
          break;
        case "align_right":
          selected.style.textAlign = "right";
          break;
      }
      updateOverlay();
      emitHtml(true);
      emitSelection();
    }

    function handleCommand(cmd) {
      if (!cmd) return;
      if (editing && cmd.type !== "load_html" && cmd.type !== "shutdown") {
        stopEditing(true);
      }
      switch (cmd.type) {
        case "load_html":
          loadHtml(cmd.html, true);
          break;
        case "insert_text":
          if (doc) insertText();
          break;
        case "insert_shape":
          if (doc) insertShape(cmd.shape);
          break;
        case "insert_image":
          if (doc) insertImage(cmd.data_url, cmd.alt);
          break;
        case "insert_table":
          if (doc) insertTable(cmd.rows, cmd.cols);
          break;
        case "table_action":
          if (doc) applyTableAction(cmd.action);
          break;
        case "set_style":
          applyStyle(cmd.prop, cmd.value);
          break;
        case "toggle_format":
          applyFormat(cmd.cmd);
          break;
        case "delete_selected":
          if (selected && selected.parentNode) {
            if (isTableCell(selected)) {
              selected.textContent = "";
              updateOverlay();
              emitHtml(true);
              emitSelection();
            } else {
              selected.remove();
              selectElement(null);
              emitHtml(true);
            }
          }
          break;
        case "duplicate_selected":
          if (selected && selected.parentNode) {
            const copy = selected.cloneNode(true);
            copy.setAttribute("data-df-placed", "1");
            const cs = doc.defaultView.getComputedStyle(selected);
            let pos = cs.position;
            if (pos === "static") pos = "relative";
            copy.style.position = pos;
            const l = parseFloat(selected.style.left) || (pos === "relative" ? 0 : selected.offsetLeft);
            const t = parseFloat(selected.style.top) || (pos === "relative" ? 0 : selected.offsetTop);
            copy.style.left = l + 16 + "px";
            copy.style.top = t + 16 + "px";
            copy.style.zIndex = String(++nextZ);
            if (uiLayer && uiLayer.parentNode === doc.body) {
              doc.body.insertBefore(copy, uiLayer);
            } else {
              doc.body.appendChild(copy);
            }
            selectElement(copy);
            emitHtml(true);
          }
          break;
        case "reorder":
          if (selected) {
            const cur = parseInt(doc.defaultView.getComputedStyle(selected).zIndex, 10) || 1;
            selected.style.zIndex =
              cmd.direction === "forward" ? String(Math.max(cur + 1, ++nextZ)) : String(Math.max(0, cur - 1));
            emitHtml(true);
            emitSelection();
          }
          break;
        case "undo":
          if (canUndo()) {
            historyIdx -= 1;
            const snapshot = history[historyIdx];
            loadHtml(snapshot, false);
            send({ type: "html_changed", html: snapshot, can_undo: canUndo(), can_redo: canRedo() });
          }
          break;
        case "redo":
          if (canRedo()) {
            historyIdx += 1;
            const snapshot = history[historyIdx];
            loadHtml(snapshot, false);
            send({ type: "html_changed", html: snapshot, can_undo: canUndo(), can_redo: canRedo() });
          }
          break;
        case "flush_html":
          if (doc) emitHtml(false);
          break;
      }
    }

    async function start(dioxusChannel) {
      channel = dioxusChannel;
      // Wait briefly for the iframe element if mount is still flushing.
      for (let i = 0; i < 20 && !document.getElementById(FRAME_ID); i++) {
        await new Promise((r) => setTimeout(r, 15));
      }
      send({ type: "ready" });
      while (true) {
        const cmd = await channel.recv();
        if (!cmd || cmd.type === "shutdown") {
          if (editing) stopEditing(true);
          break;
        }
        handleCommand(cmd);
      }
    }

    return { start };
  }

  window.__unscanmeEditor = {
    start: (dioxusChannel) => createController().start(dioxusChannel),
  };
})();
