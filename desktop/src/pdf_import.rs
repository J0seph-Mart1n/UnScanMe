//! PDF-to-Editable-HTML import: page rasterization + structured XHTML/text extraction.

use anyhow::{bail, Context, Result};
use base64::Engine;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, PartialEq)]
pub struct PdfPage {
    pub page_number: usize,
    pub png_bytes: Vec<u8>,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadedPdf {
    pub path: PathBuf,
    pub pages: Vec<PdfPage>,
    pub active_page: usize,
    pub bbox_xhtml: Option<String>,
    pub plain_text: String,
}

impl LoadedPdf {
    pub fn current_page(&self) -> Option<&PdfPage> {
        self.pages.get(self.active_page)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len().max(1)
    }
}

fn find_tool(names: &[&str]) -> Option<String> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for name in names {
            let exe = if cfg!(windows) { format!("{name}.exe") } else { (*name).to_string() };
            for dir in std::env::split_paths(&path_var) {
                let p = dir.join(&exe);
                if p.is_file() {
                    return Some(p.to_string_lossy().into_owned());
                }
            }
        }
    }

    let extra_dirs: &[&str] = if cfg!(target_os = "macos") {
        &["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"]
    } else if cfg!(windows) {
        &[
            r"C:\Program Files\poppler\Library\bin",
            r"C:\Program Files\poppler\bin",
            r"C:\ProgramData\chocolatey\bin",
        ]
    } else {
        &["/usr/bin", "/usr/local/bin", "/snap/bin"]
    };

    for dir in extra_dirs {
        for name in names {
            let exe = if cfg!(windows) { format!("{name}.exe") } else { (*name).to_string() };
            let p = Path::new(dir).join(&exe);
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

async fn render_pdf_pages(pdf_path: &Path) -> Result<Vec<PdfPage>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let work_dir = std::env::temp_dir().join(format!("df_pdf_import_{}_{stamp}", std::process::id()));
    tokio::fs::create_dir_all(&work_dir).await?;
    let prefix = work_dir.join("page");

    let mut rendered_ok = false;

    if let Some(pdftoppm) = find_tool(&["pdftoppm"]) {
        if let Ok(out) = tokio::process::Command::new(&pdftoppm)
            .args(["-png", "-r", "150"])
            .arg(pdf_path)
            .arg(&prefix)
            .output()
            .await
        {
            if out.status.success() {
                rendered_ok = true;
            }
        }
    }

    if !rendered_ok {
        if let Some(pdftocairo) = find_tool(&["pdftocairo"]) {
            if let Ok(out) = tokio::process::Command::new(&pdftocairo)
                .args(["-png", "-r", "150"])
                .arg(pdf_path)
                .arg(&prefix)
                .output()
                .await
            {
                if out.status.success() {
                    rendered_ok = true;
                }
            }
        }
    }

    if !rendered_ok {
        if let Some(mutool) = find_tool(&["mutool"]) {
            let pattern = work_dir.join("page-%04d.png");
            if let Ok(out) = tokio::process::Command::new(&mutool)
                .args(["draw", "-F", "png", "-r", "150", "-o"])
                .arg(&pattern)
                .arg(pdf_path)
                .output()
                .await
            {
                if out.status.success() {
                    rendered_ok = true;
                }
            }
        }
    }

    if !rendered_ok {
        if let Some(gs) = find_tool(&["gs", "gswin64c", "gswin32c"]) {
            let pattern = work_dir.join("page-%04d.png");
            if let Ok(out) = tokio::process::Command::new(&gs)
                .args([
                    "-dNOPAUSE",
                    "-dBATCH",
                    "-dSAFER",
                    "-sDEVICE=png16m",
                    "-r150",
                    &format!("-sOutputFile={}", pattern.display()),
                ])
                .arg(pdf_path)
                .output()
                .await
            {
                if out.status.success() {
                    rendered_ok = true;
                }
            }
        }
    }

    let mut entries = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&work_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("png") {
                entries.push(p);
            }
        }
    }
    entries.sort_by_key(|p| {
        let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("");
        let num = stem
            .rsplit(|c: char| !c.is_ascii_digit())
            .find(|s| !s.is_empty())
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(usize::MAX);
        (num, p.clone())
    });

    let mut pages = Vec::new();
    for (idx, p) in entries.into_iter().enumerate() {
        if let Ok(bytes) = tokio::fs::read(&p).await {
            if !bytes.is_empty() {
                let preview = format!(
                    "data:image/png;base64,{}",
                    base64::engine::general_purpose::STANDARD.encode(&bytes)
                );
                pages.push(PdfPage {
                    page_number: idx + 1,
                    png_bytes: bytes,
                    preview,
                });
            }
        }
    }

    let _ = tokio::fs::remove_dir_all(&work_dir).await;
    if pages.is_empty() && !rendered_ok {
        bail!(
            "Could not rasterize PDF pages. Install Poppler (`pdftoppm`) or Ghostscript (`gs`), or use Quick Extract HTML if text is selectable."
        );
    }
    Ok(pages)
}

async fn extract_bbox_and_text(pdf_path: &Path) -> (Option<String>, String) {
    let mut bbox_xhtml = None;
    let mut plain_text = String::new();

    if let Some(pdftotext) = find_tool(&["pdftotext"]) {
        if let Ok(out) = tokio::process::Command::new(&pdftotext)
            .arg("-bbox-layout")
            .arg(pdf_path)
            .arg("-")
            .output()
            .await
        {
            if out.status.success() {
                let txt = String::from_utf8_lossy(&out.stdout).into_owned();
                if txt.contains("<page") {
                    bbox_xhtml = Some(txt);
                }
            }
        }

        if let Ok(out) = tokio::process::Command::new(&pdftotext)
            .arg("-layout")
            .arg(pdf_path)
            .arg("-")
            .output()
            .await
        {
            if out.status.success() {
                plain_text = String::from_utf8_lossy(&out.stdout).into_owned();
            }
        }
    }

    if plain_text.trim().is_empty() && bbox_xhtml.is_none() {
        if let Ok(bytes) = tokio::fs::read(pdf_path).await {
            plain_text = extract_pdf_text_fallback(&bytes);
        }
    }

    (bbox_xhtml, plain_text)
}

/// Load a PDF from disk, rendering page previews and extracting layout/text metadata.
pub async fn import_pdf(path: &Path) -> Result<LoadedPdf> {
    use tokio::io::AsyncReadExt;

    if !path.is_file() {
        bail!("PDF file not found: {}", path.display());
    }
    let mut file = tokio::fs::File::open(path)
        .await
        .context("failed to read PDF file")?;
    let mut header = [0u8; 1024];
    let n = file
        .read(&mut header)
        .await
        .context("failed to read PDF file")?;
    if n < 5 || !header[..n].windows(5).any(|w| w == b"%PDF-") {
        bail!("Selected file is not a valid PDF document.");
    }

    let (pages_res, (bbox_xhtml, plain_text)) =
        tokio::join!(render_pdf_pages(path), extract_bbox_and_text(path));

    let pages = match pages_res {
        Ok(p) => p,
        Err(e) => {
            if bbox_xhtml.is_some() || !plain_text.trim().is_empty() {
                Vec::new()
            } else {
                return Err(e);
            }
        }
    };

    Ok(LoadedPdf {
        path: path.to_path_buf(),
        pages,
        active_page: 0,
        bbox_xhtml,
        plain_text,
    })
}

/// Convert a `LoadedPdf` directly into structured, editable HTML5 (offline, no AI required).
pub fn extract_editable_html(pdf: &LoadedPdf) -> String {
    if let Some(bbox) = pdf.bbox_xhtml.as_deref() {
        let html = parse_bbox_xhtml_to_html(bbox, &pdf.pages);
        if !html.trim().is_empty() {
            return html;
        }
    }
    plain_text_to_html(&pdf.plain_text, &pdf.pages)
}

// --- Structured BBox XHTML -> Editable HTML5 -------------------------------

#[derive(Debug, Clone)]
struct WordBox {
    x_min: f32,
    y_min: f32,
    x_max: f32,
    y_max: f32,
    text: String,
}

#[derive(Debug, Clone)]
struct LineBox {
    words: Vec<WordBox>,
    font_px: f32,
}

impl LineBox {
    fn gap_threshold(&self) -> f32 {
        (self.font_px * 1.15).max(14.0)
    }

    fn split_columns(&self, gap_threshold: f32) -> Vec<(f32, f32, String)> {
        let mut cols: Vec<(f32, f32, String)> = Vec::new();
        let mut cur_x_min = 0.0f32;
        let mut cur_x_max = 0.0f32;
        let mut cur_text = String::new();
        let mut prev_x_max: Option<f32> = None;

        for w in &self.words {
            if let Some(px) = prev_x_max {
                if w.x_min - px > gap_threshold && !cur_text.is_empty() {
                    cols.push((cur_x_min, cur_x_max, cur_text.trim().to_string()));
                    cur_text.clear();
                    cur_x_min = w.x_min;
                } else if !cur_text.is_empty() {
                    cur_text.push(' ');
                }
            } else {
                cur_x_min = w.x_min;
            }
            cur_text.push_str(&w.text);
            cur_x_max = w.x_max.max(cur_x_min);
            prev_x_max = Some(w.x_max);
        }
        if !cur_text.trim().is_empty() {
            cols.push((cur_x_min, cur_x_max, cur_text.trim().to_string()));
        }
        cols
    }

    fn full_text(&self) -> String {
        self.words.iter().map(|w| w.text.as_str()).collect::<Vec<_>>().join(" ")
    }

    fn y_min(&self) -> f32 {
        self.words.iter().map(|w| w.y_min).fold(f32::INFINITY, f32::min)
    }

    fn y_max(&self) -> f32 {
        self.words.iter().map(|w| w.y_max).fold(f32::NEG_INFINITY, f32::max)
    }
}

/// Group all words on a page into visual horizontal lines by overlapping Y coordinates.
/// This is essential because Poppler's `pdftotext -bbox-layout` splits multi-column
/// tables into separate `<flow>` / `<block>` elements instead of keeping row words together.
fn cluster_words_into_lines(mut words: Vec<WordBox>) -> Vec<LineBox> {
    if words.is_empty() {
        return Vec::new();
    }
    words.sort_by(|a, b| {
        let ay = (a.y_min + a.y_max) * 0.5;
        let by = (b.y_min + b.y_max) * 0.5;
        ay.total_cmp(&by).then_with(|| a.x_min.total_cmp(&b.x_min))
    });

    let mut groups: Vec<(f32, f32, Vec<WordBox>)> = Vec::new();
    for w in words {
        let w_cy = (w.y_min + w.y_max) * 0.5;
        let w_h = (w.y_max - w.y_min).max(6.0);
        if let Some((g_y_min, g_y_max, g_words)) = groups.last_mut() {
            let g_cy = (*g_y_min + *g_y_max) * 0.5;
            let g_h = (*g_y_max - *g_y_min).max(6.0);
            let tol = w_h.min(g_h) * 0.55;
            let overlaps = w.y_min <= *g_y_max - 1.0 && w.y_max >= *g_y_min + 1.0;
            if (w_cy - g_cy).abs() <= tol || overlaps {
                *g_y_min = g_y_min.min(w.y_min);
                *g_y_max = g_y_max.max(w.y_max);
                g_words.push(w);
                continue;
            }
        }
        groups.push((w.y_min, w.y_max, vec![w]));
    }

    groups
        .into_iter()
        .map(|(_, _, mut ws)| {
            ws.sort_by(|a, b| a.x_min.total_cmp(&b.x_min));
            let avg_h =
                ws.iter().map(|w| (w.y_max - w.y_min).max(6.0)).sum::<f32>() / ws.len() as f32;
            LineBox {
                words: ws,
                font_px: avg_h.clamp(9.0, 42.0),
            }
        })
        .collect()
}

fn parse_attr(tag: &str, attr: &str) -> Option<f32> {
    let key = format!(r#"{attr}=""#);
    let pos = tag.find(&key)?;
    let rest = &tag[pos + key.len()..];
    let end = rest.find('"')?;
    rest[..end].parse::<f32>().ok()
}

fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn build_table_columns(rows: &[Vec<(f32, f32, String)>]) -> Vec<(f32, f32)> {
    let mut intervals: Vec<(f32, f32)> = rows
        .iter()
        .flat_map(|r| r.iter().map(|(x0, x1, _)| (*x0, *x1)))
        .collect();
    intervals.sort_by(|a, b| a.0.total_cmp(&b.0));

    let mut cols: Vec<(f32, f32)> = Vec::new();
    for (x0, x1) in intervals {
        if let Some(last) = cols.last_mut() {
            // Merge into the same column if X-intervals overlap (within 4pt tolerance)
            if x0 <= last.1 + 4.0 {
                last.1 = last.1.max(x1);
                continue;
            }
        }
        cols.push((x0, x1));
    }
    cols
}

pub fn parse_bbox_xhtml_to_html(xhtml: &str, pages: &[PdfPage]) -> String {
    let mut parsed_pages: Vec<Vec<LineBox>> = Vec::new();
    let mut rest = xhtml;

    while let Some(page_start) = rest.find("<page ") {
        let after_start = &rest[page_start..];
        let Some(page_end_rel) = after_start.find("</page>") else {
            break;
        };
        let page_xml = &after_start[..page_end_rel];
        rest = &after_start[page_end_rel + "</page>".len()..];

        let mut page_words = Vec::new();
        let mut w_rest = page_xml;
        while let Some(w_start) = w_rest.find("<word ") {
            let w_after = &w_rest[w_start..];
            let Some(gt_rel) = w_after.find('>') else {
                break;
            };
            let attrs = &w_after[..gt_rel];
            let content_start = &w_after[gt_rel + 1..];
            let Some(w_end_rel) = content_start.find("</word>") else {
                break;
            };
            let raw_word = unescape_xml(content_start[..w_end_rel].trim());
            w_rest = &content_start[w_end_rel + "</word>".len()..];

            if raw_word.is_empty() {
                continue;
            }
            let x_min = parse_attr(attrs, "xMin").unwrap_or(0.0);
            let y_min = parse_attr(attrs, "yMin").unwrap_or(0.0);
            let x_max = parse_attr(attrs, "xMax").unwrap_or(x_min + 10.0);
            let y_max = parse_attr(attrs, "yMax").unwrap_or(y_min + 12.0);
            page_words.push(WordBox {
                x_min,
                y_min,
                x_max,
                y_max,
                text: raw_word,
            });
        }

        parsed_pages.push(cluster_words_into_lines(page_words));
    }

    if parsed_pages.is_empty() {
        return String::new();
    }

    // Estimate median body font size across all lines
    let mut all_sizes: Vec<f32> = parsed_pages
        .iter()
        .flat_map(|ls| ls.iter().map(|l| l.font_px))
        .collect();
    all_sizes.sort_by(|a, b| a.total_cmp(b));
    let median_font = if all_sizes.is_empty() {
        12.0
    } else {
        all_sizes[all_sizes.len() / 2]
    };

    let mut body_sections = Vec::new();
    for (idx, lines) in parsed_pages.iter().enumerate() {
        let mut section = String::new();
        section.push_str("<section class=\"pdf-page\">\n");

        if lines.is_empty() {
            if let Some(page_img) = pages.get(idx) {
                section.push_str(&format!(
                    "  <div style=\"text-align:center;\"><img src=\"{}\" alt=\"Imported PDF Page {}\" style=\"max-width:100%;height:auto;\"></div>\n",
                    page_img.preview,
                    idx + 1
                ));
            } else {
                section.push_str("  <p>(Blank page)</p>\n");
            }
            section.push_str("</section>");
            body_sections.push(section);
            continue;
        }

        let mut i = 0;
        while i < lines.len() {
            let cols = lines[i].split_columns(lines[i].gap_threshold());
            if cols.len() >= 2 {
                // Collect consecutive multi-column lines as an editable HTML table
                let mut rows: Vec<Vec<(f32, f32, String)>> = vec![cols];
                let mut j = i + 1;
                while j < lines.len() {
                    let next_cols = lines[j].split_columns(lines[j].gap_threshold());
                    if next_cols.len() >= 2 {
                        rows.push(next_cols);
                        j += 1;
                    } else {
                        break;
                    }
                }

                if rows.len() >= 2 {
                    let col_buckets = build_table_columns(&rows);
                    let col_count = col_buckets.len().max(2);
                    section.push_str("  <table>\n");
                    for (r_idx, row) in rows.iter().enumerate() {
                        let mut cells = vec![String::new(); col_count];
                        for (cx0, cx1, text) in row {
                            let center = (*cx0 + *cx1) * 0.5;
                            let target_idx = col_buckets
                                .iter()
                                .position(|&(b0, b1)| *cx0 <= b1 + 4.0 && *cx1 >= b0 - 4.0)
                                .unwrap_or_else(|| {
                                    col_buckets
                                        .iter()
                                        .enumerate()
                                        .min_by(|(_, a), (_, b)| {
                                            let da = ((a.0 + a.1) * 0.5 - center).abs();
                                            let db = ((b.0 + b.1) * 0.5 - center).abs();
                                            da.total_cmp(&db)
                                        })
                                        .map(|(k, _)| k)
                                        .unwrap_or(0)
                                });
                            if !cells[target_idx].is_empty() {
                                cells[target_idx].push(' ');
                            }
                            cells[target_idx].push_str(text);
                        }

                        section.push_str("    <tr>");
                        let tag = if r_idx == 0 { "th" } else { "td" };
                        for cell in &cells {
                            section.push_str(&format!("<{tag}>{}</{tag}>", escape_html(cell)));
                        }
                        section.push_str("</tr>\n");
                    }
                    section.push_str("  </table>\n");
                    i = j;
                    continue;
                }
            }

            let line = &lines[i];
            let text = line.full_text();
            if line.font_px >= median_font * 1.45 && text.len() < 120 {
                section.push_str(&format!("  <h1>{}</h1>\n", escape_html(&text)));
                i += 1;
            } else if line.font_px >= median_font * 1.2 && text.len() < 120 {
                section.push_str(&format!("  <h2>{}</h2>\n", escape_html(&text)));
                i += 1;
            } else {
                // Merge consecutive single-column lines separated by small vertical gaps into a paragraph
                let mut para = text;
                let mut prev_y = line.y_max();
                let mut j = i + 1;
                while j < lines.len() {
                    let next = &lines[j];
                    if next.split_columns(next.gap_threshold()).len() >= 2
                        && j + 1 < lines.len()
                        && lines[j + 1].split_columns(lines[j + 1].gap_threshold()).len() >= 2
                    {
                        break;
                    }
                    if next.font_px >= median_font * 1.2 {
                        break;
                    }
                    let next_y_min = next.y_min();
                    let gap = next_y_min - prev_y;
                    if gap > line.font_px * 0.9 {
                        break;
                    }
                    para.push(' ');
                    para.push_str(&next.full_text());
                    prev_y = next.y_max();
                    j += 1;
                }
                section.push_str(&format!("  <p>{}</p>\n", escape_html(&para)));
                i = j;
            }
        }

        section.push_str("</section>");
        body_sections.push(section);
    }

    wrap_imported_html(&body_sections.join("\n<hr class=\"page-sep\">\n"))
}

pub fn plain_text_to_html(text: &str, pages: &[PdfPage]) -> String {
    let raw_pages: Vec<&str> = text.split('\x0c').collect();
    let mut sections = Vec::new();

    for (idx, page_text) in raw_pages.iter().enumerate() {
        let trimmed = page_text.trim();
        if trimmed.is_empty() {
            if let Some(img) = pages.get(idx) {
                sections.push(format!(
                    "<section class=\"pdf-page\">\n  <div style=\"text-align:center;\"><img src=\"{}\" alt=\"Imported PDF Page {}\" style=\"max-width:100%;height:auto;\"></div>\n</section>",
                    img.preview,
                    idx + 1
                ));
            }
            continue;
        }

        let mut sec = String::from("<section class=\"pdf-page\">\n");
        let lines: Vec<&str> = trimmed.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();
            if line.is_empty() {
                i += 1;
                continue;
            }

            let split_spaces = |l: &str| -> Vec<String> {
                let mut cells = Vec::new();
                let mut cur = String::new();
                let mut spaces = 0usize;
                for ch in l.chars() {
                    if ch == ' ' || ch == '\t' {
                        spaces += 1;
                        if ch == '\t' {
                            spaces += 2;
                        }
                    } else {
                        if spaces >= 3 && !cur.is_empty() {
                            cells.push(cur.trim().to_string());
                            cur.clear();
                        } else if spaces > 0 && !cur.is_empty() {
                            cur.push(' ');
                        }
                        spaces = 0;
                        cur.push(ch);
                    }
                }
                if !cur.trim().is_empty() {
                    cells.push(cur.trim().to_string());
                }
                cells
            };

            let cols = split_spaces(line);
            if cols.len() >= 2 {
                let mut rows = vec![cols];
                let mut j = i + 1;
                while j < lines.len() {
                    let nl = lines[j].trim();
                    if nl.is_empty() {
                        break;
                    }
                    let nc = split_spaces(nl);
                    if nc.len() >= 2 {
                        rows.push(nc);
                        j += 1;
                    } else {
                        break;
                    }
                }
                if rows.len() >= 2 {
                    let max_c = rows.iter().map(|r| r.len()).max().unwrap_or(2);
                    sec.push_str("  <table>\n");
                    for (r_idx, row) in rows.iter().enumerate() {
                        let tag = if r_idx == 0 { "th" } else { "td" };
                        sec.push_str("    <tr>");
                        for c in 0..max_c {
                            let val = row.get(c).map(|s| s.as_str()).unwrap_or("");
                            sec.push_str(&format!("<{tag}>{}</{tag}>", escape_html(val)));
                        }
                        sec.push_str("</tr>\n");
                    }
                    sec.push_str("  </table>\n");
                    i = j;
                    continue;
                }
            }

            if i == 0 && line.len() < 80 {
                sec.push_str(&format!("  <h1>{}</h1>\n", escape_html(line)));
            } else {
                sec.push_str(&format!("  <p>{}</p>\n", escape_html(line)));
            }
            i += 1;
        }
        sec.push_str("</section>");
        sections.push(sec);
    }

    if sections.is_empty() {
        for (idx, img) in pages.iter().enumerate() {
            sections.push(format!(
                "<section class=\"pdf-page\">\n  <div style=\"text-align:center;\"><img src=\"{}\" alt=\"Imported PDF Page {}\" style=\"max-width:100%;height:auto;\"></div>\n</section>",
                img.preview,
                idx + 1
            ));
        }
    }

    wrap_imported_html(&sections.join("\n<hr class=\"page-sep\">\n"))
}

fn wrap_imported_html(body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<style>
  @page {{ size: A4; margin: 15mm; }}
  body {{
    font-family: system-ui, -apple-system, 'Segoe UI', Roboto, sans-serif;
    font-size: 11pt;
    line-height: 1.5;
    color: #111827;
    margin: 0;
    padding: 20px 24px;
    background: #ffffff;
  }}
  .pdf-page {{
    max-width: 780px;
    margin: 0 auto 24px auto;
  }}
  h1 {{ font-size: 18pt; margin: 0 0 12px 0; color: #0f172a; }}
  h2 {{ font-size: 13.5pt; margin: 16px 0 8px 0; color: #1e293b; }}
  p {{ margin: 0 0 8px 0; }}
  table {{
    border-collapse: collapse;
    width: 100%;
    margin: 12px 0;
  }}
  th, td {{
    border: 1px solid #cbd5e1;
    padding: 7px 10px;
    text-align: left;
  }}
  th {{
    background: #f1f5f9;
    font-weight: 600;
  }}
  .page-sep {{
    border: 0;
    border-top: 1px dashed #cbd5e1;
    margin: 20px 0;
    page-break-after: always;
  }}
  @media print {{
    body {{ padding: 0; }}
    .page-sep {{ visibility: hidden; margin: 0; }}
  }}
</style>
</head>
<body>
{body}
</body>
</html>"#
    )
}

/// Best-effort pure-Rust literal string scanner for uncompressed PDF text operators `(text) Tj`
/// when `pdftotext` is not installed on the system.
pub fn extract_pdf_text_fallback(bytes: &[u8]) -> String {
    let raw = String::from_utf8_lossy(bytes);
    let mut out = String::new();
    let chars: Vec<char> = raw.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '(' {
            i += 1;
            let mut buf = String::new();
            let mut depth = 1usize;
            while i < chars.len() && depth > 0 {
                let c = chars[i];
                if c == '\\' && i + 1 < chars.len() {
                    i += 1;
                    match chars[i] {
                        'n' => buf.push('\n'),
                        'r' => buf.push('\r'),
                        't' => buf.push('\t'),
                        other => buf.push(other),
                    }
                } else if c == '(' {
                    depth += 1;
                    buf.push(c);
                } else if c == ')' {
                    depth -= 1;
                    if depth > 0 {
                        buf.push(c);
                    }
                } else {
                    buf.push(c);
                }
                i += 1;
            }
            // Only collect if followed shortly by Tj or TJ or '
            let tail: String = chars[i..chars.len().min(i + 12)].iter().collect();
            if tail.contains("Tj") || tail.contains("TJ") || tail.contains('\'') {
                let trimmed = buf.trim();
                if !trimmed.is_empty() && trimmed.chars().all(|ch| !ch.is_control() || ch == '\n') {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str(trimmed);
                }
            }
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_bbox_xhtml_headings_paragraphs_and_tables() {
        let xhtml = r#"<!DOCTYPE html>
<html><body><doc>
<page width="595" height="842">
  <block>
    <line xMin="40" yMin="40" xMax="260" yMax="64">
      <word xMin="40" yMin="40" xMax="140" yMax="64">ACME</word>
      <word xMin="148" yMin="40" xMax="260" yMax="64">INVOICE</word>
    </line>
    <line xMin="40" yMin="80" xMax="320" yMax="92">
      <word xMin="40" yMin="80" xMax="110" yMax="92">Billed</word>
      <word xMin="116" yMin="80" xMax="134" yMax="92">to</word>
      <word xMin="140" yMin="80" xMax="220" yMax="92">Jane</word>
      <word xMin="226" yMin="80" xMax="270" yMax="92">&amp;</word>
      <word xMin="276" yMin="80" xMax="320" yMax="92">Co.</word>
    </line>
    <line xMin="40" yMin="130" xMax="500" yMax="142">
      <word xMin="40" yMin="130" xMax="110" yMax="142">Item</word>
      <word xMin="280" yMin="130" xMax="310" yMax="142">Qty</word>
      <word xMin="440" yMin="130" xMax="500" yMax="142">Price</word>
    </line>
    <line xMin="40" yMin="150" xMax="500" yMax="162">
      <word xMin="40" yMin="150" xMax="140" yMax="162">Hosting</word>
      <word xMin="280" yMin="150" xMax="290" yMax="162">2</word>
      <word xMin="440" yMin="150" xMax="500" yMax="162">$50.00</word>
    </line>
  </block>
</page>
</doc></body></html>"#;

        let html = parse_bbox_xhtml_to_html(xhtml, &[]);
        assert!(html.contains("<h1>ACME INVOICE</h1>"));
        assert!(html.contains("<p>Billed to Jane &amp; Co.</p>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<th>Item</th><th>Qty</th><th>Price</th>"));
        assert!(html.contains("<td>Hosting</td><td>2</td><td>$50.00</td>"));
    }

    #[test]
    fn parses_poppler_split_flows_into_aligned_table_rows() {
        // Real `pdftotext -bbox-layout` splits multi-column tables into separate <flow> blocks.
        let xhtml = r#"<!DOCTYPE html>
<html><body><doc>
  <page width="612" height="792">
    <flow>
      <block><line><word xMin="72" yMin="60" xMax="170" yMax="76">UnScanMe</word><word xMin="176" yMin="60" xMax="240" yMax="76">Receipt</word></line></block>
      <block>
        <line><word xMin="72" yMin="94" xMax="96" yMax="105">Item</word></line>
        <line><word xMin="72" yMin="112" xMax="104" yMax="123">Paper</word></line>
      </block>
    </flow>
    <flow>
      <block>
        <line><word xMin="225" yMin="94" xMax="252" yMax="105">Total</word></line>
        <line><word xMin="230" yMin="112" xMax="268" yMax="123">$12.00</word></line>
      </block>
    </flow>
  </page>
</doc></body></html>"#;

        let html = parse_bbox_xhtml_to_html(xhtml, &[]);
        assert!(html.contains("<h1>UnScanMe Receipt</h1>"), "html:\n{html}");
        assert!(html.contains("<table>"), "html:\n{html}");
        assert!(html.contains("<th>Item</th><th>Total</th>"), "html:\n{html}");
        assert!(html.contains("<td>Paper</td><td>$12.00</td>"), "html:\n{html}");
    }

    #[test]
    fn plain_text_multi_page_and_table_extraction() {
        let txt = "Monthly Report\n\nItem     Qty     Total\nApples   10      $20\x0cPage Two Summary";
        let html = plain_text_to_html(txt, &[]);
        assert!(html.contains("<h1>Monthly Report</h1>"));
        assert!(html.contains("<table>"));
        assert!(html.contains("<td>Apples</td><td>10</td><td>$20</td>"));
        assert!(html.contains("Page Two Summary"));
        assert_eq!(html.matches("<section class=\"pdf-page\">").count(), 2);
    }

    #[test]
    fn fallback_extracts_uncompressed_pdf_tj_strings() {
        let pdf_bytes = b"%PDF-1.4\nBT /F1 12 Tf 72 720 Td (Hello UnScanMe!) Tj ET\n";
        let extracted = extract_pdf_text_fallback(pdf_bytes);
        assert_eq!(extracted, "Hello UnScanMe!");
    }

    #[tokio::test]
    async fn imports_real_minimal_pdf_end_to_end() {
        let content = b"BT /F1 18 Tf 72 720 Td (UnScanMe Receipt) Tj 0 -30 Td /F1 12 Tf (Item         Total) Tj 0 -18 Td (Paper        $12.00) Tj ET\n";
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let o1 = pdf.len();
        pdf.extend_from_slice(b"1 0 obj << /Type /Catalog /Pages 2 0 R >> endobj\n");
        let o2 = pdf.len();
        pdf.extend_from_slice(b"2 0 obj << /Type /Pages /Kids [3 0 R] /Count 1 >> endobj\n");
        let o3 = pdf.len();
        pdf.extend_from_slice(b"3 0 obj << /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >> endobj\n");
        let o4 = pdf.len();
        pdf.extend_from_slice(format!("4 0 obj << /Length {} >> stream\n", content.len()).as_bytes());
        pdf.extend_from_slice(content);
        pdf.extend_from_slice(b"endstream endobj\n");
        let o5 = pdf.len();
        pdf.extend_from_slice(b"5 0 obj << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> endobj\n");
        let xref = pdf.len();
        pdf.extend_from_slice(
            format!(
                "xref\n0 6\n0000000000 65535 f \n{o1:010} 00000 n \n{o2:010} 00000 n \n{o3:010} 00000 n \n{o4:010} 00000 n \n{o5:010} 00000 n \n"
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(format!("trailer << /Size 6 /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());

        let tmp = std::env::temp_dir().join(format!("df_test_import_{}.pdf", std::process::id()));
        tokio::fs::write(&tmp, &pdf).await.unwrap();

        let imported = import_pdf(&tmp).await.unwrap();
        let _ = tokio::fs::remove_file(&tmp).await;

        assert_eq!(imported.page_count(), 1);
        let html = extract_editable_html(&imported);
        assert!(html.contains("<h1>UnScanMe Receipt</h1>"), "html was:\n{html}");
        assert!(html.contains("<table>"), "html was:\n{html}");
        assert!(html.contains("<th>Item</th><th>Total</th>"), "html was:\n{html}");
        assert!(html.contains("<td>Paper</td><td>$12.00</td>"), "html was:\n{html}");
    }
}
