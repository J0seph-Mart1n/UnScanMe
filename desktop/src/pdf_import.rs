//! PDF-to-Editable-HTML import: page rasterization, structured XHTML/text extraction, and offline OCR.

use anyhow::{bail, Context, Result};
use base64::Engine;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static OCR_SEQ: AtomicU64 = AtomicU64::new(0);

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
    pub cached_has_selectable_text: bool,
    pub cached_image_only_pages: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExtractedPage {
    Structured(String),
    NeedsOcr { page_index: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PdfExtractionPlan {
    pub pages: Vec<ExtractedPage>,
}

impl PdfExtractionPlan {
    pub fn has_any_text(&self) -> bool {
        self.pages
            .iter()
            .any(|p| matches!(p, ExtractedPage::Structured(_)))
    }

    pub fn image_only_pages(&self) -> Vec<usize> {
        self.pages
            .iter()
            .filter_map(|p| match p {
                ExtractedPage::NeedsOcr { page_index } => Some(*page_index),
                ExtractedPage::Structured(_) => None,
            })
            .collect()
    }

    pub fn render_structured_only(&self) -> Option<String> {
        let structured: Vec<String> = self
            .pages
            .iter()
            .filter_map(|p| match p {
                ExtractedPage::Structured(html) => Some(html.clone()),
                ExtractedPage::NeedsOcr { .. } => None,
            })
            .collect();
        if structured.is_empty() {
            None
        } else {
            Some(combine_html_pages(&structured))
        }
    }
}

impl LoadedPdf {
    pub fn from_parts(
        path: PathBuf,
        pages: Vec<PdfPage>,
        bbox_xhtml: Option<String>,
        plain_text: String,
    ) -> Self {
        let mut pdf = Self {
            path,
            pages,
            active_page: 0,
            bbox_xhtml,
            plain_text,
            cached_has_selectable_text: false,
            cached_image_only_pages: 0,
        };
        let plan = plan_pdf_extraction(&pdf);
        pdf.cached_has_selectable_text = plan.has_any_text();
        pdf.cached_image_only_pages = plan.image_only_pages().len();
        pdf
    }

    pub fn current_page(&self) -> Option<&PdfPage> {
        self.pages.get(self.active_page)
    }

    pub fn page_count(&self) -> usize {
        self.pages.len().max(1)
    }

    pub fn has_selectable_text(&self) -> bool {
        self.cached_has_selectable_text
    }

    pub fn image_only_page_count(&self) -> usize {
        self.cached_image_only_pages
    }
}

fn find_tool(names: &[&str]) -> Option<String> {
    if let Some(path_var) = std::env::var_os("PATH") {
        for name in names {
            let exe = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                (*name).to_string()
            };
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
            r"C:\Program Files\Tesseract-OCR",
            r"C:\ProgramData\chocolatey\bin",
        ]
    } else {
        &["/usr/bin", "/usr/local/bin", "/snap/bin"]
    };

    for dir in extra_dirs {
        for name in names {
            let exe = if cfg!(windows) {
                format!("{name}.exe")
            } else {
                (*name).to_string()
            };
            let p = Path::new(dir).join(&exe);
            if p.is_file() {
                return Some(p.to_string_lossy().into_owned());
            }
        }
    }
    None
}

struct TempDirGuard(PathBuf);

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct TempFileGuard(PathBuf);

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

struct AbortOnDrop(Vec<tokio::task::AbortHandle>);

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        for handle in &self.0 {
            handle.abort();
        }
    }
}

async fn render_pdf_pages(pdf_path: &Path) -> Result<Vec<PdfPage>> {
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let work_dir = std::env::temp_dir().join(format!("df_pdf_import_{}_{stamp}", std::process::id()));
    let _guard = TempDirGuard(work_dir.clone());
    tokio::fs::create_dir_all(&work_dir).await?;
    let prefix = work_dir.join("page");

    let mut rendered_ok = false;

    if let Some(pdftoppm) = find_tool(&["pdftoppm"]) {
        if let Ok(out) = tokio::process::Command::new(&pdftoppm)
            .kill_on_drop(true)
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
                .kill_on_drop(true)
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
                .kill_on_drop(true)
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
                .kill_on_drop(true)
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

    if pages.is_empty() && !rendered_ok {
        bail!(
            "Could not rasterize PDF pages. Install Poppler (`pdftoppm`) or Ghostscript (`gs`), or extract if text is selectable."
        );
    }
    Ok(pages)
}

async fn extract_bbox_and_text(pdf_path: &Path) -> (Option<String>, String) {
    let mut bbox_xhtml = None;
    let mut plain_text = String::new();

    if let Some(pdftotext) = find_tool(&["pdftotext"]) {
        if let Ok(out) = tokio::process::Command::new(&pdftotext)
            .kill_on_drop(true)
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

        // Skip redundant plain-text pass if bbox-layout already extracted words.
        let has_bbox_words = bbox_xhtml.as_deref().is_some_and(|x| x.contains("<word "));
        if !has_bbox_words {
            if let Ok(out) = tokio::process::Command::new(&pdftotext)
                .kill_on_drop(true)
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

    Ok(LoadedPdf::from_parts(
        path.to_path_buf(),
        pages,
        bbox_xhtml,
        plain_text,
    ))
}

/// Build an explicit per-page extraction plan so image-only pages are routed to AI/OCR
/// instead of being wrapped into flat non-editable `<img>` elements.
pub fn plan_pdf_extraction(pdf: &LoadedPdf) -> PdfExtractionPlan {
    let parsed_bbox_pages = pdf
        .bbox_xhtml
        .as_deref()
        .map(parse_bbox_pages)
        .unwrap_or_default();
    let median_font = estimate_median_font(&parsed_bbox_pages);

    let plain_pages: Vec<&str> = if pdf.plain_text.trim().is_empty() {
        Vec::new()
    } else {
        pdf.plain_text.split('\x0c').collect()
    };

    let total_pages = parsed_bbox_pages
        .len()
        .max(plain_pages.len())
        .max(pdf.pages.len())
        .max(1);

    let mut pages = Vec::with_capacity(total_pages);
    for idx in 0..total_pages {
        if let Some(lines) = parsed_bbox_pages.get(idx).filter(|ls| !ls.is_empty()) {
            pages.push(ExtractedPage::Structured(lines_to_section_html(
                lines,
                median_font,
            )));
            continue;
        }
        if let Some(trimmed) = plain_pages
            .get(idx)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            pages.push(ExtractedPage::Structured(plain_text_section_html(trimmed)));
            continue;
        }
        pages.push(ExtractedPage::NeedsOcr { page_index: idx });
    }

    PdfExtractionPlan { pages }
}

/// Convert any selectable-text pages of `LoadedPdf` directly into structured HTML5.
/// Never emits non-editable full-page `<img>` wrappers.
#[cfg(test)]
pub fn extract_editable_html(pdf: &LoadedPdf) -> String {
    plan_pdf_extraction(pdf)
        .render_structured_only()
        .unwrap_or_default()
}

// --- Structured WordBox / LineBox -> Editable HTML5 ------------------------

#[derive(Debug, Clone)]
pub struct WordBox {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
    pub text: String,
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
        self.words
            .iter()
            .map(|w| w.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn y_min(&self) -> f32 {
        self.words
            .iter()
            .map(|w| w.y_min)
            .fold(f32::INFINITY, f32::min)
    }

    fn y_max(&self) -> f32 {
        self.words
            .iter()
            .map(|w| w.y_max)
            .fold(f32::NEG_INFINITY, f32::max)
    }
}

/// Group all words on a page into visual horizontal lines by overlapping Y coordinates.
/// Essential because Poppler's `pdftotext -bbox-layout` splits multi-column tables
/// into separate `<flow>` / `<block>` elements, and OCR emits unordered/row words.
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
            if x0 <= last.1 + 4.0 {
                last.1 = last.1.max(x1);
                continue;
            }
        }
        cols.push((x0, x1));
    }
    cols
}

fn parse_bbox_pages(xhtml: &str) -> Vec<Vec<LineBox>> {
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

    parsed_pages
}

fn estimate_median_font(pages: &[Vec<LineBox>]) -> f32 {
    let mut all_sizes: Vec<f32> = pages
        .iter()
        .flat_map(|ls| ls.iter().map(|l| l.font_px))
        .collect();
    all_sizes.sort_by(|a, b| a.total_cmp(b));
    if all_sizes.is_empty() {
        12.0
    } else {
        all_sizes[all_sizes.len() / 2]
    }
}

fn lines_to_section_html(lines: &[LineBox], median_font: f32) -> String {
    let mut section = String::from("<section class=\"pdf-page\">\n");
    let mut i = 0;
    while i < lines.len() {
        let cols = lines[i].split_columns(lines[i].gap_threshold());
        if cols.len() >= 2 {
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
            let mut para = text;
            let mut prev_y = line.y_max();
            let mut j = i + 1;
            while j < lines.len() {
                let next = &lines[j];
                if next.split_columns(next.gap_threshold()).len() >= 2
                    && j + 1 < lines.len()
                    && lines[j + 1]
                        .split_columns(lines[j + 1].gap_threshold())
                        .len()
                        >= 2
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
    section
}

#[cfg(test)]
pub fn parse_bbox_xhtml_to_html(xhtml: &str, _pages: &[PdfPage]) -> String {
    let parsed_pages = parse_bbox_pages(xhtml);
    if parsed_pages.is_empty() {
        return String::new();
    }
    let median_font = estimate_median_font(&parsed_pages);
    let body_sections: Vec<String> = parsed_pages
        .iter()
        .filter(|lines| !lines.is_empty())
        .map(|lines| lines_to_section_html(lines, median_font))
        .collect();
    if body_sections.is_empty() {
        String::new()
    } else {
        wrap_imported_html(&body_sections.join("\n<hr class=\"page-sep\">\n"))
    }
}

fn plain_text_section_html(trimmed: &str) -> String {
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
    sec
}

#[cfg(test)]
pub fn plain_text_to_html(text: &str, _pages: &[PdfPage]) -> String {
    let sections: Vec<String> = text
        .split('\x0c')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(plain_text_section_html)
        .collect();
    if sections.is_empty() {
        String::new()
    } else {
        wrap_imported_html(&sections.join("\n<hr class=\"page-sep\">\n"))
    }
}

// --- Offline Tesseract TSV OCR Fallback ------------------------------------

/// Parse Tesseract `--psm 6 tsv` output into positioned `WordBox` entries.
pub fn parse_tesseract_tsv_to_words(tsv: &str) -> Vec<WordBox> {
    let mut words = Vec::new();
    for line in tsv.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 12 {
            continue;
        }
        let Ok(level) = parts[0].trim().parse::<u32>() else {
            continue;
        };
        if level != 5 {
            continue;
        }
        let conf = parts[10].trim().parse::<f32>().unwrap_or(-1.0);
        if conf < 0.0 {
            continue;
        }
        let text = parts[11..].join("\t").trim().to_string();
        if text.is_empty() {
            continue;
        }
        let Ok(left) = parts[6].trim().parse::<f32>() else {
            continue;
        };
        let Ok(top) = parts[7].trim().parse::<f32>() else {
            continue;
        };
        let Ok(width) = parts[8].trim().parse::<f32>() else {
            continue;
        };
        let Ok(height) = parts[9].trim().parse::<f32>() else {
            continue;
        };
        if width <= 0.0 || height <= 0.0 {
            continue;
        }
        words.push(WordBox {
            x_min: left,
            y_min: top,
            x_max: left + width,
            y_max: top + height,
            text,
        });
    }
    words
}

/// Convert Tesseract TSV OCR output into an editable `<section class="pdf-page">...</section>`.
pub fn tesseract_tsv_to_section_html(tsv: &str) -> Option<String> {
    let words = parse_tesseract_tsv_to_words(tsv);
    if words.is_empty() {
        return None;
    }
    let lines = cluster_words_into_lines(words);
    if lines.is_empty() {
        return None;
    }
    let median_font = estimate_median_font(std::slice::from_ref(&lines));
    Some(lines_to_section_html(&lines, median_font))
}

/// Returns `true` if a local `tesseract` binary is available on this system.
pub fn has_tesseract() -> bool {
    find_tool(&["tesseract"]).is_some()
}

/// Run local `tesseract` CLI (if installed) on rasterized page PNG bytes to build editable HTML.
pub async fn ocr_png_bytes_to_section(png_bytes: &[u8]) -> Result<Option<String>> {
    let Some(tesseract) = find_tool(&["tesseract"]) else {
        return Ok(None);
    };
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let seq = OCR_SEQ.fetch_add(1, Ordering::Relaxed);
    let tmp_png = std::env::temp_dir().join(format!(
        "unscanme_ocr_{}_{}_{}.png",
        std::process::id(),
        stamp,
        seq
    ));
    let _guard = TempFileGuard(tmp_png.clone());
    tokio::fs::write(&tmp_png, png_bytes)
        .await
        .context("failed to stage temporary image for Tesseract OCR")?;

    let out = tokio::process::Command::new(&tesseract)
        .kill_on_drop(true)
        .env("OMP_THREAD_LIMIT", "1")
        .arg(&tmp_png)
        .args(["stdout", "--psm", "6", "tsv"])
        .output()
        .await
        .context("failed to invoke Tesseract OCR")?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("Tesseract OCR failed: {}", err.trim());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    Ok(tesseract_tsv_to_section_html(&stdout))
}

/// Run offline Tesseract OCR in parallel (up to 4 concurrent pages) across `(page_index, png_bytes)`,
/// reporting `(completed_pages, total_pages)` progress after each batch completes.
pub async fn ocr_pages_parallel_with_progress<F>(
    pages: &[(usize, Vec<u8>)],
    mut on_progress: F,
) -> Result<Vec<(usize, Option<String>)>>
where
    F: FnMut(usize, usize),
{
    if pages.is_empty() || !has_tesseract() {
        return Ok(pages.iter().map(|(idx, _)| (*idx, None)).collect());
    }

    let total = pages.len();
    let mut out = Vec::with_capacity(total);
    for chunk in pages.chunks(4) {
        let mut abort_guard = AbortOnDrop(Vec::with_capacity(chunk.len()));
        let mut handles = Vec::with_capacity(chunk.len());
        for (page_idx, bytes) in chunk {
            let idx = *page_idx;
            let png = bytes.clone();
            let jh = tokio::spawn(async move {
                let res = ocr_png_bytes_to_section(&png).await;
                (idx, res)
            });
            abort_guard.0.push(jh.abort_handle());
            handles.push(jh);
        }
        for handle in handles {
            let (idx, res) = handle.await.context("OCR task panicked")?;
            out.push((idx, res?));
        }
        abort_guard.0.clear();
        on_progress(out.len(), total);
    }
    out.sort_by_key(|(idx, _)| *idx);
    Ok(out)
}

#[cfg(test)]
pub async fn ocr_pages_parallel(pages: &[(usize, Vec<u8>)]) -> Result<Vec<(usize, Option<String>)>> {
    ocr_pages_parallel_with_progress(pages, |_, _| {}).await
}

// --- Multi-Page HTML Combiner ----------------------------------------------

fn find_ci(haystack: &str, needle: &str) -> Option<usize> {
    let h_low = haystack.to_ascii_lowercase();
    h_low.find(&needle.to_ascii_lowercase())
}

fn extract_tag_inner(doc: &str, tag: &str) -> Option<String> {
    let open_pat = format!("<{tag}");
    let close_pat = format!("</{tag}>");
    let start = find_ci(doc, &open_pat)?;
    let gt_rel = doc[start..].find('>')?;
    let content_start = start + gt_rel + 1;
    let end = find_ci(&doc[content_start..], &close_pat)?;
    Some(doc[content_start..content_start + end].trim().to_string())
}

fn extract_all_styles(doc: &str) -> Vec<String> {
    let mut styles = Vec::new();
    let mut rest = doc;
    while let Some(start) = find_ci(rest, "<style") {
        let after = &rest[start..];
        let Some(gt_rel) = after.find('>') else {
            break;
        };
        let body_start = start + gt_rel + 1;
        let Some(close_rel) = find_ci(&rest[body_start..], "</style>") else {
            break;
        };
        let css = rest[body_start..body_start + close_rel].trim();
        if !css.is_empty() {
            styles.push(css.to_string());
        }
        rest = &rest[body_start + close_rel + "</style>".len()..];
    }
    styles
}

/// Combine mixed structured `<section>` fragments and/or full AI-generated HTML5 documents
/// into a single self-contained editable HTML5 document.
pub fn combine_html_pages(pages: &[String]) -> String {
    if pages.is_empty() {
        return String::new();
    }
    if pages.len() == 1 {
        let single = pages[0].trim();
        if find_ci(single, "<!doctype").is_some() || find_ci(single, "<html").is_some() {
            return pages[0].clone();
        }
        return wrap_imported_html(single);
    }

    let mut extra_styles = Vec::new();
    let mut body_parts = Vec::new();

    for page in pages {
        let trimmed = page.trim();
        if trimmed.is_empty() {
            continue;
        }
        if find_ci(trimmed, "<html").is_some() || find_ci(trimmed, "<body").is_some() {
            for css in extract_all_styles(trimmed) {
                if !extra_styles.contains(&css) {
                    extra_styles.push(css);
                }
            }
            if let Some(body) = extract_tag_inner(trimmed, "body") {
                if !body.is_empty() {
                    body_parts.push(format!("<section class=\"pdf-page\">\n{body}\n</section>"));
                }
            }
        } else {
            body_parts.push(trimmed.to_string());
        }
    }

    if body_parts.is_empty() {
        return String::new();
    }

    wrap_imported_html_with_extra_styles(
        &body_parts.join("\n<hr class=\"page-sep\">\n"),
        &extra_styles,
    )
}

fn wrap_imported_html(body: &str) -> String {
    wrap_imported_html_with_extra_styles(body, &[])
}

fn wrap_imported_html_with_extra_styles(body: &str, extra_styles: &[String]) -> String {
    let extra_css_block = if extra_styles.is_empty() {
        String::new()
    } else {
        format!("\n<style>\n{}\n</style>", extra_styles.join("\n\n"))
    };
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
</style>{extra_css_block}
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

    #[test]
    fn scanned_pdf_never_emits_flat_page_image_and_marks_needs_ocr() {
        let pdf = LoadedPdf::from_parts(
            PathBuf::from("scanned_invoice.pdf"),
            vec![PdfPage {
                page_number: 1,
                png_bytes: vec![1, 2, 3],
                preview: "data:image/png;base64,AQID".into(),
            }],
            Some(r#"<doc><page width="595" height="842"></page></doc>"#.into()),
            String::new(),
        );

        assert!(!pdf.has_selectable_text());
        assert_eq!(pdf.image_only_page_count(), 1);
        let plan = plan_pdf_extraction(&pdf);
        assert_eq!(plan.image_only_pages(), vec![0]);
        let direct = extract_editable_html(&pdf);
        assert!(!direct.contains("<img"), "unexpected image wrapper:\n{direct}");
        assert!(direct.is_empty());
    }

    #[tokio::test]
    async fn ocr_pages_parallel_handles_empty_input() {
        let res = ocr_pages_parallel(&[]).await.unwrap();
        assert!(res.is_empty());
    }

    #[tokio::test]
    async fn abort_on_drop_cancels_spawned_tasks() {
        let jh = tokio::spawn(std::future::pending::<()>());
        let guard = AbortOnDrop(vec![jh.abort_handle()]);
        drop(guard);
        let err = jh.await.expect_err("task should have been aborted on drop");
        assert!(err.is_cancelled());
    }

    #[test]
    fn tesseract_tsv_parses_headings_paragraphs_and_tables() {
        let tsv = "\
level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext
5\t1\t1\t1\t1\t1\t60\t40\t120\t28\t95\tINVOICE
5\t1\t1\t1\t2\t1\t60\t90\t50\t14\t96\tBilled
5\t1\t1\t1\t2\t2\t116\t90\t45\t14\t94\tToday
5\t1\t2\t1\t1\t1\t60\t140\t60\t14\t95\tItem
5\t1\t2\t1\t1\t2\t320\t140\t60\t14\t95\tPrice
5\t1\t2\t1\t2\t1\t60\t164\t80\t14\t94\tScanner
5\t1\t2\t1\t2\t2\t320\t164\t65\t14\t92\t$99.00";

        let section = tesseract_tsv_to_section_html(tsv).expect("section expected");
        assert!(section.contains("<h1>INVOICE</h1>"), "section:\n{section}");
        assert!(section.contains("<p>Billed Today</p>"), "section:\n{section}");
        assert!(section.contains("<table>"), "section:\n{section}");
        assert!(section.contains("<th>Item</th><th>Price</th>"), "section:\n{section}");
        assert!(section.contains("<td>Scanner</td><td>$99.00</td>"), "section:\n{section}");
    }

    #[test]
    fn combine_html_pages_merges_styles_and_bodies() {
        let p1 = "<!DOCTYPE html><html><head><style>.a{color:#111;}</style></head><body><h1>Page One</h1></body></html>".to_string();
        let p2 = "<!DOCTYPE html><html><head><style>.b{color:#222;}</style></head><body><p>Page Two</p></body></html>".to_string();
        let combined = combine_html_pages(&[p1, p2]);
        assert!(combined.contains(".a{color:#111;}"));
        assert!(combined.contains(".b{color:#222;}"));
        assert!(combined.contains("<h1>Page One</h1>"));
        assert!(combined.contains("<p>Page Two</p>"));
        assert!(combined.contains("<hr class=\"page-sep\">"));
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
