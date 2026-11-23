// DocForge Visual Canvas Editor Controller
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
      default: return el.tagName ? el.tagName.toLowerCase() : "el";
    }
  }

  function isTextEditable(el) {
    if (!el) return false;
    const kind = elementKind(el);
    if (kind === "image" || kind === "line" || kind === "arrow") return false;
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
    return "<!DOCTYPE html>\n" + clone.outerHTML;
  }

  function createController() {
    let channel = null;
    let frame = null;
    let doc = null;
    let selected = null;
    let editing = null;
    let uiLayer = null;
    let selectBox = null;
    let badgeEl = null;
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

    function updateOverlay() {
      if (!selectBox || !doc) return;
      if (!selected || !doc.body.contains(selected)) {
        selected = null;
        selectBox.style.display = "none";
        return;
      }
      const rect = selected.getBoundingClientRect();
      const scrollX = doc.defaultView ? doc.defaultView.scrollX : 0;
      const scrollY = doc.defaultView ? doc.defaultView.scrollY : 0;
      selectBox.style.display = "block";
      selectBox.style.left = rect.left + scrollX + "px";
      selectBox.style.top = rect.top + scrollY + "px";
      selectBox.style.width = Math.max(4, rect.width) + "px";
      selectBox.style.height = Math.max(4, rect.height) + "px";
      selectBox.classList.toggle("is-editing", editing === selected);

      if (badgeEl) {
        if (dragState && dragState.mode === "move") {
          badgeEl.textContent = `${Math.round(rect.left + scrollX)}, ${Math.round(rect.top + scrollY)}`;
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
          #${UI_ID} .df-select-box {
            position: absolute; display: none; box-sizing: border-box;
            border: 1.5px solid #3b6ef5; pointer-events: none;
          }
          #${UI_ID} .df-select-box.is-editing {
            border-color: #22c55e; border-style: dashed;
          }
          #${UI_ID} .df-handle {
            position: absolute; width: 9px; height: 9px; background: #ffffff;
            border: 1.5px solid #3b6ef5; border-radius: 2px;
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
          #${UI_ID} .df-badge {
            position: absolute; left: 0; top: -22px; display: none;
            padding: 1px 6px; border-radius: 4px; font: 600 11px/16px system-ui, sans-serif;
            background: #111827; color: #f8fafc; white-space: nowrap;
            box-shadow: 0 2px 6px rgba(0,0,0,.35);
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

        uiLayer.appendChild(selectBox);
        doc.body.appendChild(uiLayer);
      } else {
        uiLayer = doc.getElementById(UI_ID);
        selectBox = uiLayer.querySelector(".df-select-box");
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
      selected = el;
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

    function resolveSelectableTarget(target) {
      if (!target || !doc) return null;
      if (isEditorNode(target)) return null;
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
      if (target === doc.body || target === doc.documentElement) return null;
      return target;
    }

    function startMove(e, el) {
      const win = doc.defaultView;
      const cs = win.getComputedStyle(el);
      const isPlaced = el.getAttribute("data-df-placed") === "1";
      let pos = cs.position;
      if (pos === "static") {
        pos = isPlaced ? "absolute" : "relative";
        el.style.position = pos;
      }
      const startLeft = parseFloat(el.style.left) || (pos === "relative" ? 0 : el.offsetLeft);
      const startTop = parseFloat(el.style.top) || (pos === "relative" ? 0 : el.offsetTop);

      dragState = {
        mode: "move",
        el,
        startX: e.clientX,
        startY: e.clientY,
        startLeft,
        startTop,
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
      if (!dragState) return;
      const dx = e.clientX - dragState.startX;
      const dy = e.clientY - dragState.startY;
      if (!dragState.moved && Math.abs(dx) < 2 && Math.abs(dy) < 2) return;
      dragState.moved = true;

      const free = e.altKey;
      const el = dragState.el;
      if (dragState.mode === "move") {
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
      const didMove = dragState.moved;
      dragState = null;
      updateOverlay();
      if (didMove) {
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
      } else if (e.key === "Delete" || e.key === "Backspace") {
        e.preventDefault();
        handleCommand({ type: "delete_selected" });
      } else if (["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(e.key)) {
        e.preventDefault();
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
        const target = resolveSelectableTarget(e.target);
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
        const target = resolveSelectableTarget(e.target);
        if (target && isTextEditable(target)) {
          e.preventDefault();
          startEditing(target);
        }
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
        el.style.background = "#e0e7ff";
        el.style.border = "2px solid #3b6ef5";
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
        el.style.background = "#dbeafe";
        el.style.border = "2px solid #2563eb";
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
        case "set_style":
          applyStyle(cmd.prop, cmd.value);
          break;
        case "toggle_format":
          applyFormat(cmd.cmd);
          break;
        case "delete_selected":
          if (selected && selected.parentNode) {
            selected.remove();
            selectElement(null);
            emitHtml(true);
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
        if (!cmd || cmd.type === "shutdown") break;
        handleCommand(cmd);
      }
    }

    return { start };
  }

  window.__docforgeEditor = {
    start: (dioxusChannel) => createController().start(dioxusChannel),
  };
})();
