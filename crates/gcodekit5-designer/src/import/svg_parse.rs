//! Low-level SVG parsing helpers for [`super::SvgImporter`].
//!
//! Provides a small XML tag tokenizer (enough for SVG geometry: start, end and
//! self-closing tags with quoted attributes; comments, processing instructions,
//! doctypes and CDATA are skipped), an affine transform type with a parser for the
//! SVG `transform` attribute (`matrix`, `translate`, `scale`, `rotate`, `skewX`,
//! `skewY`), and the root `viewBox` → millimetre mapping.
//!
//! No external dependencies beyond `std`.

use crate::model::Point;

/// 2D affine transform in SVG `matrix(a b c d e f)` form:
/// `x' = a·x + c·y + e`, `y' = b·x + d·y + f`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Affine {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

/// Tolerance used when classifying a transform's linear part.
const CLASSIFY_EPS: f64 = 1e-9;

impl Affine {
    /// The identity transform.
    pub const IDENTITY: Affine = Affine {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// Translation by `(tx, ty)`.
    pub fn translate(tx: f64, ty: f64) -> Self {
        Affine {
            e: tx,
            f: ty,
            ..Self::IDENTITY
        }
    }

    /// Scale by `(sx, sy)` about the origin.
    pub fn scale(sx: f64, sy: f64) -> Self {
        Affine {
            a: sx,
            d: sy,
            ..Self::IDENTITY
        }
    }

    /// Rotation by `deg` degrees about the origin (positive turns +X towards +Y).
    pub fn rotate(deg: f64) -> Self {
        let (s, c) = deg.to_radians().sin_cos();
        Affine {
            a: c,
            b: s,
            c: -s,
            d: c,
            e: 0.0,
            f: 0.0,
        }
    }

    /// Returns `self · other`: the transform that applies `other` first, then `self`.
    pub fn mul(&self, other: &Affine) -> Affine {
        Affine {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            e: self.a * other.e + self.c * other.f + self.e,
            f: self.b * other.e + self.d * other.f + self.f,
        }
    }

    /// Applies the transform to a point.
    pub fn apply(&self, p: Point) -> Point {
        Point::new(
            self.a * p.x + self.c * p.y + self.e,
            self.b * p.x + self.d * p.y + self.f,
        )
    }

    /// Converts to a lyon transform (single precision).
    pub fn to_lyon(self) -> lyon::math::Transform {
        lyon::math::Transform::new(
            self.a as f32,
            self.b as f32,
            self.c as f32,
            self.d as f32,
            self.e as f32,
            self.f as f32,
        )
    }

    /// True when the linear part has no rotation or skew (axes stay axis-aligned).
    pub fn is_axis_aligned(&self) -> bool {
        self.b.abs() < CLASSIFY_EPS && self.c.abs() < CLASSIFY_EPS
    }

    /// If the linear part is a rotation times a uniform positive scale, returns
    /// `(scale, rotation_degrees)`.
    pub fn as_similarity(&self) -> Option<(f64, f64)> {
        let s = (self.a * self.a + self.b * self.b).sqrt();
        let tol = CLASSIFY_EPS.max(s * 1e-9);
        if s > 0.0 && (self.a - self.d).abs() < tol && (self.b + self.c).abs() < tol {
            Some((s, self.b.atan2(self.a).to_degrees()))
        } else {
            None
        }
    }
}

/// Splits a number list (`"1, 2 -3e2"`) into values; also handles signs that
/// directly follow a number (`"10-5"`).
pub(crate) fn parse_numbers(s: &str) -> Vec<f64> {
    let mut out = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let ch = bytes[i];
        if ch.is_ascii_whitespace() || ch == b',' {
            i += 1;
            continue;
        }
        let start = i;
        if ch == b'+' || ch == b'-' {
            i += 1;
        }
        let mut seen_dot = false;
        let mut seen_exp = false;
        while i < bytes.len() {
            let c = bytes[i];
            if c.is_ascii_digit() {
                i += 1;
            } else if c == b'.' && !seen_dot && !seen_exp {
                seen_dot = true;
                i += 1;
            } else if (c == b'e' || c == b'E') && !seen_exp && i > start {
                seen_exp = true;
                i += 1;
                if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
            } else {
                break;
            }
        }
        if i == start {
            // Unparseable character: skip it.
            i += 1;
            continue;
        }
        if let Ok(v) = s[start..i].parse::<f64>() {
            out.push(v);
        }
    }
    out
}

/// Parses an SVG `transform` attribute into a single affine transform.
///
/// The functions are composed left to right as the SVG specification requires
/// (`"translate(10) scale(2)"` scales first, then translates). Unknown functions
/// or malformed argument lists are ignored.
pub(crate) fn parse_transform(s: &str) -> Affine {
    let mut result = Affine::IDENTITY;
    let mut rest = s;
    while let Some(open) = rest.find('(') {
        let name = rest[..open]
            .trim_matches(|c: char| c.is_whitespace() || c == ',')
            .to_ascii_lowercase();
        let Some(close_rel) = rest[open..].find(')') else {
            break;
        };
        let args = parse_numbers(&rest[open + 1..open + close_rel]);
        rest = &rest[open + close_rel + 1..];

        let t = match (name.as_str(), args.as_slice()) {
            ("matrix", [a, b, c, d, e, f]) => Some(Affine {
                a: *a,
                b: *b,
                c: *c,
                d: *d,
                e: *e,
                f: *f,
            }),
            ("translate", [tx]) => Some(Affine::translate(*tx, 0.0)),
            ("translate", [tx, ty]) => Some(Affine::translate(*tx, *ty)),
            ("scale", [s]) => Some(Affine::scale(*s, *s)),
            ("scale", [sx, sy]) => Some(Affine::scale(*sx, *sy)),
            ("rotate", [deg]) => Some(Affine::rotate(*deg)),
            ("rotate", [deg, cx, cy]) => Some(
                Affine::translate(*cx, *cy)
                    .mul(&Affine::rotate(*deg))
                    .mul(&Affine::translate(-cx, -cy)),
            ),
            ("skewx", [deg]) => Some(Affine {
                c: deg.to_radians().tan(),
                ..Affine::IDENTITY
            }),
            ("skewy", [deg]) => Some(Affine {
                b: deg.to_radians().tan(),
                ..Affine::IDENTITY
            }),
            _ => {
                tracing::debug!(function = %name, "Ignoring unsupported SVG transform");
                None
            }
        };
        if let Some(t) = t {
            result = result.mul(&t);
        }
    }
    result
}

/// Kind of an XML tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TagKind {
    /// `<name ...>`
    Start,
    /// `</name>`
    End,
    /// `<name .../>`
    Empty,
}

/// One XML tag with its (namespace-stripped) name and attributes.
#[derive(Debug, Clone)]
pub(crate) struct XmlTag {
    pub name: String,
    pub kind: TagKind,
    pub attrs: Vec<(String, String)>,
}

impl XmlTag {
    /// Returns the value of attribute `name`, if present.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    /// Returns attribute `name` parsed as a number (a trailing unit such as `px`
    /// is ignored), if present and valid.
    pub fn attr_f64(&self, name: &str) -> Option<f64> {
        self.attr(name)
            .and_then(|v| parse_numbers(v).first().copied())
    }
}

/// Strips a namespace prefix (`svg:rect` → `rect`).
fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

/// Parses the attribute section of a tag (`x="1" y='2'`).
fn parse_attrs(s: &str) -> Vec<(String, String)> {
    let mut attrs = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        while i < bytes.len() && (bytes[i].is_ascii_whitespace() || bytes[i] == b'/') {
            i += 1;
        }
        let name_start = i;
        while i < bytes.len()
            && !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'='
            && bytes[i] != b'/'
        {
            i += 1;
        }
        let name = &s[name_start..i];
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            // Attribute without a value (not valid XML); skip it.
            if i == name_start {
                i += 1;
            }
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let quote = bytes[i];
        if quote != b'"' && quote != b'\'' {
            continue;
        }
        i += 1;
        let val_start = i;
        while i < bytes.len() && bytes[i] != quote {
            i += 1;
        }
        let value = &s[val_start..i.min(bytes.len())];
        i += 1;
        if !name.is_empty() {
            attrs.push((local_name(name).to_string(), value.to_string()));
        }
    }
    attrs
}

/// Splits SVG/XML text into tags, skipping text content, comments, processing
/// instructions, doctypes and CDATA sections.
pub(crate) fn tokenize(content: &str) -> Vec<XmlTag> {
    let mut tags = Vec::new();
    let mut pos = 0;
    while let Some(rel) = content[pos..].find('<') {
        let start = pos + rel;
        let rest = &content[start..];
        let skip_to = |end: &str| rest.find(end).map(|e| start + e + end.len());
        if rest.starts_with("<!--") {
            match skip_to("-->") {
                Some(p) => pos = p,
                None => break,
            }
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            match skip_to("]]>") {
                Some(p) => pos = p,
                None => break,
            }
            continue;
        }
        if rest.starts_with("<?") {
            match skip_to("?>") {
                Some(p) => pos = p,
                None => break,
            }
            continue;
        }
        if rest.starts_with("<!") {
            match skip_to(">") {
                Some(p) => pos = p,
                None => break,
            }
            continue;
        }

        // Find the closing '>' outside quoted attribute values.
        let bytes = rest.as_bytes();
        let mut i = 1;
        let mut quote: Option<u8> = None;
        while i < bytes.len() {
            match (quote, bytes[i]) {
                (None, b'"') | (None, b'\'') => quote = Some(bytes[i]),
                (Some(q), c) if c == q => quote = None,
                (None, b'>') => break,
                _ => {}
            }
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let inner = &rest[1..i];
        pos = start + i + 1;

        if let Some(name) = inner.strip_prefix('/') {
            tags.push(XmlTag {
                name: local_name(name.trim()).to_string(),
                kind: TagKind::End,
                attrs: Vec::new(),
            });
            continue;
        }
        let self_closing = inner.trim_end().ends_with('/');
        let name_end = inner
            .find(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(inner.len());
        let name = local_name(&inner[..name_end]).to_string();
        if name.is_empty() {
            continue;
        }
        tags.push(XmlTag {
            name,
            kind: if self_closing {
                TagKind::Empty
            } else {
                TagKind::Start
            },
            attrs: parse_attrs(&inner[name_end..]),
        });
    }
    tags
}

/// Parses an SVG length (`"210mm"`, `"8.5in"`, `"100px"`, `"100"`) into millimetres.
///
/// Supported units: `mm`, `cm`, `in`, `pt`, `pc`, `px` (96 dpi). A unitless
/// length is treated as millimetres, which keeps the importer's historical
/// behaviour of mapping one user unit to 1 mm. Percentages are not resolvable
/// without a viewport and yield `None`.
pub(crate) fn parse_length_mm(s: &str) -> Option<f64> {
    let s = s.trim();
    let split = s
        .find(|c: char| c.is_ascii_alphabetic() || c == '%')
        .unwrap_or(s.len());
    let value: f64 = s[..split].trim().parse().ok()?;
    let factor = match s[split..].trim().to_ascii_lowercase().as_str() {
        "" | "mm" => 1.0,
        "cm" => 10.0,
        "in" => 25.4,
        "pt" => 25.4 / 72.0,
        "pc" => 25.4 / 6.0,
        "px" => 25.4 / 96.0,
        _ => return None,
    };
    Some(value * factor)
}

/// Computes the transform from root user units (viewBox coordinates) to millimetres,
/// and the document size in millimetres, from the root `<svg>` tag.
///
/// `width`/`height` give the physical size; `viewBox` gives the user-unit extent that
/// maps onto it, honouring `preserveAspectRatio` (default `xMidYMid meet`). Without a
/// viewBox, user units are scaled by the width/height unit only (`px` → 25.4/96 mm).
pub(crate) fn root_transform(svg: &XmlTag) -> (Affine, (f64, f64)) {
    let width_attr = svg.attr("width");
    let height_attr = svg.attr("height");
    let width_mm = width_attr.and_then(parse_length_mm);
    let height_mm = height_attr.and_then(parse_length_mm);
    let viewbox = svg
        .attr("viewBox")
        .map(parse_numbers)
        .filter(|v| v.len() == 4 && v[2] > 0.0 && v[3] > 0.0);

    let Some(vb) = viewbox else {
        // No viewBox: user units are CSS pixels when the size is given in px etc.
        // Derive the unit factor from the width attribute's unit.
        let unit_factor = width_attr
            .or(height_attr)
            .and_then(|w| {
                let n = parse_numbers(w).first().copied()?;
                let mm = parse_length_mm(w)?;
                (n.abs() > 1e-12).then_some(mm / n)
            })
            .unwrap_or(1.0);
        let w = width_mm.unwrap_or(100.0);
        let h = height_mm.unwrap_or(100.0);
        return (Affine::scale(unit_factor, unit_factor), (w, h));
    };
    let (min_x, min_y, vb_w, vb_h) = (vb[0], vb[1], vb[2], vb[3]);

    // Physical size; a missing dimension follows the viewBox aspect ratio.
    let (w, h) = match (width_mm, height_mm) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) => (w, w * vb_h / vb_w),
        (None, Some(h)) => (h * vb_w / vb_h, h),
        (None, None) => (vb_w, vb_h),
    };
    let (sx, sy) = (w / vb_w, h / vb_h);

    let par = svg.attr("preserveAspectRatio").unwrap_or("xMidYMid meet");
    let mut parts = par.split_whitespace();
    let align = parts.next().unwrap_or("xMidYMid");
    let slice = parts.next() == Some("slice");

    let t = if align == "none" {
        Affine::scale(sx, sy).mul(&Affine::translate(-min_x, -min_y))
    } else {
        let s = if slice { sx.max(sy) } else { sx.min(sy) };
        let free_x = w - vb_w * s;
        let free_y = h - vb_h * s;
        let tx = if align.contains("xMid") {
            free_x / 2.0
        } else if align.contains("xMax") {
            free_x
        } else {
            0.0
        };
        let ty = if align.contains("YMid") {
            free_y / 2.0
        } else if align.contains("YMax") {
            free_y
        } else {
            0.0
        };
        Affine::translate(tx, ty)
            .mul(&Affine::scale(s, s))
            .mul(&Affine::translate(-min_x, -min_y))
    };
    (t, (w, h))
}
