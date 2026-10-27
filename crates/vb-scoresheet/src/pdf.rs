//! A PDF page as positioned text and drawn circles: just enough of the
//! content stream to read a form laid out by a program (eScoresheet). Text
//! runs are where each `Tj` lands; circles are small closed curves (the
//! scoresheet circles a captain's number with one).

use lopdf::content::Operation;
use lopdf::{Document, Object, ObjectId};

use crate::Error;

/// One piece of text as drawn. `y` is measured from the top of the page.
#[derive(Debug, Clone, PartialEq)]
pub struct Run {
    pub x: f64,
    pub y: f64,
    pub size: f64,
    pub bold: bool,
    pub italic: bool,
    pub text: String,
}

/// The box around a drawn circle, `y` from the top.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}

impl Circle {
    pub fn contains(&self, x: f64, y: f64) -> bool {
        (self.x0..=self.x1).contains(&x) && (self.y0..=self.y1).contains(&y)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Page {
    pub width: f64,
    pub height: f64,
    pub runs: Vec<Run>,
    pub circles: Vec<Circle>,
}

type Matrix = [f64; 6];
const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `a` then `b`, as PDF composes matrices (`a × b`).
fn mul(a: &Matrix, b: &Matrix) -> Matrix {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

fn apply(m: &Matrix, x: f64, y: f64) -> (f64, f64) {
    (m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5])
}

fn num(o: &Object) -> f64 {
    match o {
        Object::Integer(i) => *i as f64,
        Object::Real(r) => f64::from(*r),
        _ => 0.0,
    }
}

fn nums(op: &Operation) -> Vec<f64> {
    op.operands.iter().map(num).collect()
}

/// A page's value for `key`, looking up the page tree (`MediaBox` is
/// often on the parent).
fn inherited<'a>(doc: &'a Document, page: ObjectId, key: &[u8]) -> Option<&'a Object> {
    let mut id = page;
    for _ in 0..32 {
        let dict = doc.get_dictionary(id).ok()?;
        if let Ok(v) = dict.get(key) {
            return Some(v);
        }
        id = dict.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

/// Text in a simple font: WinAnsi, as eScoresheet writes it.
fn decode(bytes: &[u8]) -> String {
    let (text, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes);
    text.into_owned()
}

/// The first page of a PDF.
pub fn first_page(bytes: &[u8]) -> Result<Page, Error> {
    let doc = Document::load_mem(bytes).map_err(|e| Error::Pdf(e.to_string()))?;
    let (_, &page) = doc.get_pages().iter().next().ok_or(Error::Empty)?;
    let media: Vec<f64> = inherited(&doc, page, b"MediaBox")
        .and_then(|o| doc.dereference(o).ok())
        .and_then(|(_, o)| o.as_array().ok())
        .map(|a| a.iter().map(num).collect())
        .unwrap_or_else(|| vec![0.0, 0.0, 842.0, 595.0]);
    let (x0, y0) = (
        media.first().copied().unwrap_or(0.0),
        media.get(1).copied().unwrap_or(0.0),
    );
    let width = media.get(2).copied().unwrap_or(842.0) - x0;
    let height = media.get(3).copied().unwrap_or(595.0) - y0;
    // Font resource name → (bold, italic), from the base font's name.
    let styles: Vec<(Vec<u8>, bool, bool)> = doc
        .get_page_fonts(page)
        .map_err(|e| Error::Pdf(e.to_string()))?
        .into_iter()
        .map(|(name, font)| {
            let base = font
                .get(b"BaseFont")
                .ok()
                .and_then(|b| b.as_name().ok())
                .map(|b| String::from_utf8_lossy(b).to_ascii_lowercase())
                .unwrap_or_default();
            (
                name,
                base.contains("bold"),
                base.contains("italic") || base.contains("oblique"),
            )
        })
        .collect();
    let content = doc
        .get_and_decode_page_content(page)
        .map_err(|e| Error::Pdf(e.to_string()))?;

    let mut out = Page {
        width,
        height,
        ..Page::default()
    };
    let flip = |(x, y): (f64, f64)| (x - x0, height - (y - y0));
    let mut ctm = IDENTITY;
    let mut saved: Vec<Matrix> = Vec::new();
    // Text state.
    let (mut tm, mut line) = (IDENTITY, IDENTITY);
    let (mut size, mut leading) = (0.0, 0.0);
    let (mut bold, mut italic) = (false, false);
    // The path being built: its points in page space, and whether it curves.
    let mut path: Vec<(f64, f64)> = Vec::new();
    let mut curved = false;

    for op in &content.operations {
        let v = nums(op);
        let at = |i: usize| v.get(i).copied().unwrap_or(0.0);
        match op.operator.as_str() {
            "q" => saved.push(ctm),
            "Q" => ctm = saved.pop().unwrap_or(IDENTITY),
            "cm" => ctm = mul(&[at(0), at(1), at(2), at(3), at(4), at(5)], &ctm),
            "BT" => {
                tm = IDENTITY;
                line = IDENTITY;
            }
            "Tf" => {
                size = at(1);
                let name = op.operands.first().and_then(|o| o.as_name().ok());
                let style = name.and_then(|n| styles.iter().find(|(k, _, _)| k.as_slice() == n));
                (bold, italic) = style.map_or((false, false), |(_, b, i)| (*b, *i));
            }
            "TL" => leading = at(0),
            "Tm" => {
                line = [at(0), at(1), at(2), at(3), at(4), at(5)];
                tm = line;
            }
            "Td" | "TD" => {
                if op.operator == "TD" {
                    leading = -at(1);
                }
                line = mul(&[1.0, 0.0, 0.0, 1.0, at(0), at(1)], &line);
                tm = line;
            }
            "T*" => {
                line = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &line);
                tm = line;
            }
            "Tj" | "TJ" | "'" | "\"" => {
                if matches!(op.operator.as_str(), "'" | "\"") {
                    line = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -leading], &line);
                    tm = line;
                }
                let mut text = String::new();
                for o in &op.operands {
                    match o {
                        Object::String(b, _) => text.push_str(&decode(b)),
                        Object::Array(parts) => {
                            for p in parts {
                                match p {
                                    Object::String(b, _) => text.push_str(&decode(b)),
                                    // A big gap between parts reads as a space.
                                    q if num(q) < -200.0 => text.push(' '),
                                    _ => {}
                                }
                            }
                        }
                        _ => {}
                    }
                }
                let m = mul(&tm, &ctm);
                let (x, y) = flip(apply(&m, 0.0, 0.0));
                let scale = (m[2] * m[2] + m[3] * m[3]).sqrt();
                out.runs.push(Run {
                    x,
                    y,
                    size: size * scale,
                    bold,
                    italic,
                    text,
                });
            }
            // Each subpath that curves is kept on its own: a rounded box
            // is four corner curves joined by straight sides.
            "m" => {
                flush(&mut path, &mut curved, &mut out.circles);
                path.push(flip(apply(&ctm, at(0), at(1))));
            }
            "l" => path.push(flip(apply(&ctm, at(0), at(1)))),
            "c" => {
                curved = true;
                for i in [0, 2, 4] {
                    path.push(flip(apply(&ctm, at(i), at(i + 1))));
                }
            }
            "v" | "y" => {
                curved = true;
                for i in [0, 2] {
                    path.push(flip(apply(&ctm, at(i), at(i + 1))));
                }
            }
            "re" | "h" => flush(&mut path, &mut curved, &mut out.circles),
            "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                flush(&mut path, &mut curved, &mut out.circles);
            }
            _ => {}
        }
    }
    out.circles = marks(std::mem::take(&mut out.circles));
    Ok(out)
}

/// Keeps the box of a subpath that curves, and starts a new one.
fn flush(path: &mut Vec<(f64, f64)>, curved: &mut bool, out: &mut Vec<Circle>) {
    if *curved && !path.is_empty() {
        let xs = path.iter().map(|p| p.0);
        let ys = path.iter().map(|p| p.1);
        out.push(Circle {
            x0: xs.clone().fold(f64::INFINITY, f64::min),
            x1: xs.fold(f64::NEG_INFINITY, f64::max),
            y0: ys.clone().fold(f64::INFINITY, f64::min),
            y1: ys.fold(f64::NEG_INFINITY, f64::max),
        });
    }
    path.clear();
    *curved = false;
}

/// Curves that nearly touch make one mark (a box with rounded corners is
/// drawn as four corner curves); only marks the size of a number's are kept, not
/// a frame's.
fn marks(mut curves: Vec<Circle>) -> Vec<Circle> {
    // A rounded box's corners are up to about 6 points apart.
    const GAP: f64 = 6.0;
    let near = |a: &Circle, b: &Circle| {
        a.x0 <= b.x1 + GAP && b.x0 <= a.x1 + GAP && a.y0 <= b.y1 + GAP && b.y0 <= a.y1 + GAP
    };
    let mut merged: Vec<Circle> = Vec::new();
    while let Some(mut c) = curves.pop() {
        // Grow `c` until nothing left touches it.
        loop {
            let before = curves.len();
            curves.retain(|o| {
                if near(&c, o) {
                    c = Circle {
                        x0: c.x0.min(o.x0),
                        y0: c.y0.min(o.y0),
                        x1: c.x1.max(o.x1),
                        y1: c.y1.max(o.y1),
                    };
                    false
                } else {
                    true
                }
            });
            if curves.len() == before {
                break;
            }
        }
        merged.push(c);
    }
    merged
        .into_iter()
        .filter(|c| (3.0..=25.0).contains(&(c.x1 - c.x0)) && (3.0..=25.0).contains(&(c.y1 - c.y0)))
        .collect()
}
