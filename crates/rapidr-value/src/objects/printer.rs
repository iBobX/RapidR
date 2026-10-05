//! PRINTER (RapidQ manual, Appendix A): the global object programs print
//! with — `BeginDoc`, drawing and `TextOut` on the page, `NewPage`,
//! `EndDoc`. RapidR keeps each page's drawing and makes a PDF of the
//! document ([`Printer::pdf`]); `EndDoc` hands it to the runtime's print hook
//! (the desktop sends it to a printer with `lp`, the web to the browser's
//! print dialog — `super::set_print_hook`).
//!
//! Pages are A4 at 300 dpi: 2480 × 3508 printer pixels (swapped with
//! `Orientation = poLandscape`), the coordinates programs draw in and
//! `PageWidth` / `PageHeight`. Text is Helvetica (the PDF standard font, so
//! nothing is embedded) at the Font's size in points, its widths from
//! Helvetica's metrics (`TextWidth` / `TextHeight`).

use super::bitmap::Bitmap;
use super::font::Font;
use crate::{v_int, v_str, Value};

/// Printer pixels per inch.
pub const DPI: f64 = 300.0;
/// A4 in printer pixels.
pub const PAGE_WIDTH: i64 = 2480;
pub const PAGE_HEIGHT: i64 = 3508;
/// Most drawing kept per page, and pages per document.
const MAX_OPS: usize = 200_000;
const MAX_PAGES: usize = 1_000;

#[derive(Clone, Debug)]
pub enum PageOp {
    Line(i64, i64, i64, i64, u32),
    Rect(i64, i64, i64, i64, u32),
    Fill(i64, i64, i64, i64, u32),
    Ellipse(i64, i64, i64, i64, u32, Option<u32>),
    Pixel(i64, i64, u32),
    /// x, y (top left), text, font, background.
    Text(i64, i64, String, Font, Option<u32>),
    /// Left, top, width, height, the image.
    Image(i64, i64, i64, i64, Bitmap),
}

/// A finished document for the print hook.
#[derive(Clone, Debug)]
pub struct PrintJob {
    pub pdf: Vec<u8>,
    /// The printer chosen (`Printers(PrinterIndex)`), "" for the default.
    pub printer: String,
    pub copies: i64,
    pub title: String,
}

#[derive(Clone, Debug)]
pub struct Printer {
    pub pages: Vec<Vec<PageOp>>,
    pub printing: bool,
    pub aborted: bool,
    pub font: Font,
    pub landscape: bool,
    pub copies: i64,
    pub title: String,
    pub printers: Vec<String>,
    pub index: i64,
}

impl Default for Printer {
    fn default() -> Self {
        let printers = super::printer_names();
        Printer { pages: Vec::new(), printing: false, aborted: false, font: Font::default(), landscape: false, copies: 1, title: String::new(), printers, index: 0 }
    }
}

/// Helvetica's widths (1/1000 em) for ' ' … '~'.
const HELVETICA: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278, 278, 556, 556, 556, 556, 556, 556, 556, 556, 556, 556,
    278, 278, 584, 584, 584, 556, 1015, 667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, 722, 778, 667, 778, 722,
    667, 611, 722, 667, 944, 667, 667, 611, 278, 278, 278, 469, 556, 333, 556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500,
    222, 833, 556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, 334, 260, 334, 584,
];

/// Printer pixels per point.
fn px_per_pt() -> f64 {
    DPI / 72.0
}

impl Printer {
    pub fn page_size(&self) -> (i64, i64) {
        if self.landscape {
            (PAGE_HEIGHT, PAGE_WIDTH)
        } else {
            (PAGE_WIDTH, PAGE_HEIGHT)
        }
    }

    /// Text size in points (the Font's, at least 1).
    fn points(&self) -> f64 {
        self.font.size.clamp(1, 1_000) as f64
    }

    /// `TextWidth(s)` in printer pixels.
    pub fn text_width(&self, s: &str) -> i64 {
        let units: u32 = s.chars().map(|c| HELVETICA.get((c as usize).wrapping_sub(32)).copied().unwrap_or(556) as u32).sum();
        (units as f64 / 1000.0 * self.points() * px_per_pt()).round() as i64
    }

    /// `TextHeight(s)` in printer pixels: the line height.
    pub fn text_height(&self) -> i64 {
        (self.points() * 1.15 * px_per_pt()).round() as i64
    }

    fn push(&mut self, op: PageOp) {
        if self.pages.is_empty() {
            self.pages.push(Vec::new());
        }
        let page = self.pages.last_mut().expect("a page");
        if page.len() < MAX_OPS {
            page.push(op);
        }
    }

    /// Keeps a drawing on the current page (the document starts with the
    /// first drawing if BeginDoc wasn't called).
    pub fn draw(&mut self, op: PageOp) {
        self.printing = true;
        self.push(op);
    }

    pub fn get(&self, prop: &str) -> Option<Value> {
        let flag = |b: bool| v_int(if b { -1 } else { 0 });
        let (w, h) = self.page_size();
        Some(match prop {
            "pagewidth" => v_int(w),
            "pageheight" => v_int(h),
            "pagenumber" => v_int(self.pages.len().max(1) as i64),
            "printing" => flag(self.printing),
            "aborted" => flag(self.aborted),
            "orientation" => v_int(i64::from(self.landscape)),
            "copies" => v_int(self.copies),
            "title" => v_str(&self.title),
            "printerindex" => v_int(self.index),
            "printerscount" => v_int(self.printers.len() as i64),
            "capabilities.copies" | "capabilities.orientation" | "capabilities.collate" => flag(true),
            "handle" => v_int(0),
            p => return p.strip_prefix("font.").and_then(|p| self.font.get(p)),
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> bool {
        match prop {
            "orientation" => self.landscape = val.to_i64() == 1,
            "copies" => self.copies = val.to_i64().clamp(1, 999),
            "title" => self.title = val.to_string_val(),
            "printerindex" => self.index = val.to_i64().clamp(-1, self.printers.len() as i64 - 1).max(0),
            p => return p.strip_prefix("font.").is_some_and(|p| self.font.set(p, val)),
        }
        true
    }

    /// Methods that need nothing but the printer. `None`: not one of them.
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let n = |i: usize| args.get(i).map_or(0, Value::to_i64);
        let c = |i: usize| crate::objects::color_bgr(n(i));
        let optional = |i: usize| args.get(i).map(Value::to_i64).filter(|v| *v >= 0 || (*v as u32) & 0xFF00_0000 == 0x8000_0000).map(crate::objects::color_bgr);
        match method {
            "begindoc" => {
                self.pages = vec![Vec::new()];
                self.printing = true;
                self.aborted = false;
            }
            "newpage" => {
                if self.pages.len() < MAX_PAGES {
                    self.pages.push(Vec::new());
                }
            }
            "abort" => {
                self.pages.clear();
                self.printing = false;
                self.aborted = true;
            }
            "line" => self.draw(PageOp::Line(n(0), n(1), n(2), n(3), c(4))),
            "rectangle" => self.draw(PageOp::Rect(n(0), n(1), n(2), n(3), c(4))),
            "fillrect" => self.draw(PageOp::Fill(n(0), n(1), n(2), n(3), c(4))),
            "circle" => self.draw(PageOp::Ellipse(n(0), n(1), n(2), n(3), c(4), optional(5))),
            "roundrect" => self.draw(PageOp::Fill(n(0), n(1), n(2), n(3), c(6))),
            "pset" => self.draw(PageOp::Pixel(n(0), n(1), c(2))),
            // TextOut(x, y, text, color, background (-1: transparent)).
            "textout" => {
                let mut font = self.font.clone();
                if args.len() > 3 {
                    font.color = crate::objects::color_bgr(n(3)) as i64;
                }
                let text = args.get(2).map(|v| v.to_string_val()).unwrap_or_default();
                self.draw(PageOp::Text(n(0), n(1), text, font, optional(4)));
            }
            "textwidth" => return Some(v_int(self.text_width(&args.first().map(|v| v.to_string_val()).unwrap_or_default()))),
            "textheight" => return Some(v_int(self.text_height())),
            "printers" => {
                let i = usize::try_from(n(0)).ok();
                return Some(v_str(i.and_then(|i| self.printers.get(i)).map_or("", |s| s.as_str())));
            }
            "capabilities.copies" | "capabilities.orientation" | "capabilities.collate" => return Some(v_int(-1)),
            _ => return None,
        }
        Some(Value::Null)
    }

    /// Ends the document: the job to print (`None` if nothing was drawn).
    pub fn end_doc(&mut self) -> Option<PrintJob> {
        let printing = std::mem::take(&mut self.printing);
        if !printing || self.pages.iter().all(Vec::is_empty) {
            self.pages.clear();
            return None;
        }
        let pdf = self.pdf();
        self.pages.clear();
        let printer = usize::try_from(self.index).ok().and_then(|i| self.printers.get(i)).cloned().unwrap_or_default();
        Some(PrintJob { pdf, printer, copies: self.copies, title: self.title.clone() })
    }

    /// The document as a PDF: one page per page, drawing in printer pixels
    /// (y down) turned into points (y up).
    pub fn pdf(&self) -> Vec<u8> {
        let (w, h) = self.page_size();
        let pt = |px: i64| px as f64 * 72.0 / DPI;
        let (wp, hp) = (pt(w), pt(h));
        let x = |px: i64| format!("{:.2}", pt(px));
        let y = |px: i64| format!("{:.2}", hp - pt(px));
        let rgb = |c: u32| format!("{:.3} {:.3} {:.3}", (c & 0xFF) as f64 / 255.0, ((c >> 8) & 0xFF) as f64 / 255.0, ((c >> 16) & 0xFF) as f64 / 255.0);
        let mut objects: Vec<Vec<u8>> = Vec::new();
        // 1 catalog, 2 pages, 3–6 fonts; then per page: page, content, images.
        objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
        objects.push(Vec::new());
        for f in ["Helvetica", "Helvetica-Bold", "Helvetica-Oblique", "Helvetica-BoldOblique"] {
            objects.push(format!("<< /Type /Font /Subtype /Type1 /BaseFont /{f} /Encoding /WinAnsiEncoding >>").into_bytes());
        }
        let mut kids = Vec::new();
        let pages: Vec<&Vec<PageOp>> = if self.pages.is_empty() { Vec::new() } else { self.pages.iter().collect() };
        for page in pages {
            let page_obj = objects.len() + 1;
            let content_obj = page_obj + 1;
            objects.push(Vec::new());
            objects.push(Vec::new());
            let mut images = Vec::new();
            let mut s = String::from("1 w\n");
            for op in page {
                match op {
                    PageOp::Line(x1, y1, x2, y2, c) => s += &format!("{} RG {} {} m {} {} l S\n", rgb(*c), x(*x1), y(*y1), x(*x2), y(*y2)),
                    PageOp::Rect(x1, y1, x2, y2, c) => {
                        s += &format!("{} RG {} {} {:.2} {:.2} re S\n", rgb(*c), x(*x1.min(x2)), y(*y1.max(y2)), pt((x2 - x1).abs()), pt((y2 - y1).abs()))
                    }
                    PageOp::Fill(x1, y1, x2, y2, c) => {
                        s += &format!("{} rg {} {} {:.2} {:.2} re f\n", rgb(*c), x(*x1.min(x2)), y(*y1.max(y2)), pt((x2 - x1).abs()), pt((y2 - y1).abs()))
                    }
                    PageOp::Ellipse(x1, y1, x2, y2, c, fill) => {
                        let (cx, cy) = (pt(x1 + x2) / 2.0, hp - pt(y1 + y2) / 2.0);
                        let (rx, ry) = (pt((x2 - x1).abs()) / 2.0, pt((y2 - y1).abs()) / 2.0);
                        let (kx, ky) = (rx * 0.5523, ry * 0.5523);
                        let path = format!(
                            "{:.2} {cy:.2} m {:.2} {:.2} {:.2} {:.2} {cx:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {cy:.2} c {:.2} {:.2} {:.2} {:.2} {cx:.2} {:.2} c {:.2} {:.2} {:.2} {:.2} {:.2} {cy:.2} c",
                            cx + rx, cx + rx, cy + ky, cx + kx, cy + ry, cy + ry, cx - kx, cy + ry, cx - rx, cy + ky, cx - rx,
                            cx - rx, cy - ky, cx - kx, cy - ry, cy - ry, cx + kx, cy - ry, cx + rx, cy - ky, cx + rx
                        );
                        match fill {
                            Some(f) => s += &format!("{} rg {} RG {path} B\n", rgb(*f), rgb(*c)),
                            None => s += &format!("{} RG {path} S\n", rgb(*c)),
                        }
                    }
                    PageOp::Pixel(px, py, c) => s += &format!("{} rg {} {} {:.2} {:.2} re f\n", rgb(*c), x(*px), y(*py + 1), pt(1), pt(1)),
                    PageOp::Text(tx, ty, text, font, bg) => {
                        let size = font.size.clamp(1, 1_000) as f64;
                        let bold = font.styles & 1 != 0;
                        let italic = font.styles & 2 != 0;
                        let f = 1 + usize::from(bold) + 2 * usize::from(italic);
                        if let Some(bg) = bg {
                            let pr = Printer { font: font.clone(), ..self.clone() };
                            let (tw, th) = (pr.text_width(text), pr.text_height());
                            s += &format!("{} rg {} {} {:.2} {:.2} re f\n", rgb(*bg), x(*tx), y(ty + th), pt(tw), pt(th));
                        }
                        // The top of the text at y: its baseline an ascent below.
                        let baseline = hp - pt(*ty) - size * 0.718;
                        s += &format!("BT /F{f} {size:.1} Tf {} rg {} {baseline:.2} Td ({}) Tj ET\n", rgb(crate::objects::color_bgr(font.color)), x(*tx), pdf_string(text));
                    }
                    PageOp::Image(ix, iy, iw, ih, b) => {
                        let name = format!("Im{}", images.len() + 1);
                        s += &format!("q {:.2} 0 0 {:.2} {} {} cm /{name} Do Q\n", pt(*iw), pt(*ih), x(*ix), y(iy + ih));
                        images.push((name, b.clone()));
                    }
                }
            }
            let mut xobjects = String::new();
            for (name, b) in images {
                let rgbs: Vec<u8> = b.img.pixels.iter().flat_map(|c| [(*c & 0xFF) as u8, ((*c >> 8) & 0xFF) as u8, ((*c >> 16) & 0xFF) as u8]).collect();
                let mut obj = format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Length {} >>\nstream\n",
                    b.img.width, b.img.height, rgbs.len()
                )
                .into_bytes();
                obj.extend_from_slice(&rgbs);
                obj.extend_from_slice(b"\nendstream");
                objects.push(obj);
                xobjects += &format!("/{name} {} 0 R ", objects.len());
            }
            let mut content = format!("<< /Length {} >>\nstream\n", s.len()).into_bytes();
            content.extend_from_slice(s.as_bytes());
            content.extend_from_slice(b"\nendstream");
            objects[content_obj - 1] = content;
            objects[page_obj - 1] = format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {wp:.2} {hp:.2}] /Contents {content_obj} 0 R /Resources << /Font << /F1 3 0 R /F2 4 0 R /F3 5 0 R /F4 6 0 R >> /XObject << {xobjects}>> >> >>"
            )
            .into_bytes();
            kids.push(format!("{page_obj} 0 R"));
        }
        objects[1] = format!("<< /Type /Pages /Kids [{}] /Count {} >>", kids.join(" "), kids.len()).into_bytes();
        let mut out = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::new();
        for (i, obj) in objects.iter().enumerate() {
            offsets.push(out.len());
            out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
            out.extend_from_slice(obj);
            out.extend_from_slice(b"\nendobj\n");
        }
        let xref = out.len();
        out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
        for o in offsets {
            out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
        }
        out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
        out
    }
}

/// Text as a PDF string in WinAnsiEncoding (Latin-1 as is; other
/// characters as `?`), escaped.
fn pdf_string(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        match c {
            '(' | ')' | '\\' => {
                out.push('\\');
                out.push(c);
            }
            ' '..='~' => out.push(c),
            c if (c as u32) >= 0xA0 && (c as u32) <= 0xFF => out += &format!("\\{:03o}", c as u32),
            _ => out.push('?'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_and_pdf() {
        let mut p = Printer { printers: vec!["Office".into()], ..Printer::default() };
        p.set("font.size", &v_int(12));
        assert_eq!(p.text_width("MM"), (2.0 * 833.0 / 1000.0 * 12.0 * DPI / 72.0_f64).round() as i64);
        assert_eq!(p.get("pagewidth").unwrap().to_i64(), 2480);
        p.set("orientation", &v_int(1));
        assert_eq!(p.get("pagewidth").unwrap().to_i64(), 3508);
        p.set("orientation", &v_int(0));
        p.call("begindoc", &[]);
        p.call("textout", &[v_int(100), v_int(100), v_str("Hello (world)"), v_int(0xFF), v_int(-1)]);
        p.call("line", &[v_int(0), v_int(0), v_int(2480), v_int(3508), v_int(0)]);
        p.call("newpage", &[]);
        p.call("circle", &[v_int(10), v_int(10), v_int(110), v_int(60), v_int(0), v_int(0xFF00)]);
        assert_eq!(p.get("pagenumber").unwrap().to_i64(), 2);
        let job = p.end_doc().unwrap();
        assert_eq!(job.printer, "Office");
        let pdf = String::from_utf8_lossy(&job.pdf);
        assert!(pdf.starts_with("%PDF-1.4") && pdf.trim_end().ends_with("%%EOF"));
        assert!(pdf.contains("/Count 2"));
        assert!(pdf.contains("(Hello \\(world\\)) Tj"));
        assert!(pdf.contains("1.000 0.000 0.000 rg"), "red text");
        // Nothing drawn: nothing to print.
        p.call("begindoc", &[]);
        assert!(p.end_doc().is_none());
    }
}
