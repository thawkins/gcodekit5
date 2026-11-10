//! # File Import Module
//!
//! Provides functionality to import design files (SVG, DXF) into Designer shapes.
//!
//! This module provides importers for converting external file formats into Designer shapes.
//! Includes full SVG path parsing and DXF entity conversion.
//!
//! Supports:
//! - File format detection and validation
//! - SVG path parsing (lines, circles, rectangles, ellipses, paths)
//! - DXF entity conversion (lines, circles, arcs, polylines)
//! - Coordinate system transformation
//! - Scale and offset adjustment

use crate::dxf_parser::{DxfEntity, DxfFile, DxfParser};
use crate::model::{
    DesignCircle as Circle, DesignEllipse as Ellipse, DesignLine as Line, DesignPath as PathShape,
    DesignRectangle as Rectangle, DesignerShape, Point, Shape,
};
use crate::model3d::{Mesh3D, Model3DImporter};
use anyhow::{anyhow, Result};
use lyon::math::{point, vector};
use lyon::path::Path;

mod svg_parse;

/// Represents an imported design from a file
#[derive(Debug)]
pub struct ImportedDesign {
    /// Imported shapes as trait objects
    pub shapes: Vec<Shape>,
    /// Original file dimensions (width, height)
    pub dimensions: (f64, f64),
    /// Source file format
    pub format: FileFormat,
    /// Number of layers imported
    pub layer_count: usize,
    /// Optional 3D mesh for 3D models
    pub mesh_3d: Option<Mesh3D>,
    /// Entity types found in the source file that were parsed but discarded
    /// (e.g. ACIS solids), mapped to occurrence count
    pub unsupported_entities: Vec<(String, usize)>,
    /// Values that were not numbers and were replaced by a default (DXF)
    pub replaced_numbers: usize,
    /// Per-shape (elevation, thickness) in mm, aligned by index with `shapes`;
    /// (0.0, 0.0) when the source entity carried no Z metadata
    pub z_metadata: Vec<(f64, f64)>,
}

/// Supported import file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// SVG (Scalable Vector Graphics)
    Svg,
    /// DXF (Drawing Exchange Format)
    Dxf,
    /// STL (STereoLithography) - 3D model format
    Stl,
}

/// SVG importer for converting SVG files to Designer shapes
///
/// Currently provides basic framework for SVG import.
/// Full implementation requires SVG parsing library integration.
pub struct SvgImporter {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

enum ImportedShape {
    Rect(Rectangle),
    Circle(Circle),
    Line(Line),
    Ellipse(Ellipse),
    Path(PathShape),
}

impl ImportedShape {
    fn bounds(&self) -> (f64, f64, f64, f64) {
        match self {
            Self::Rect(s) => s.bounds(),
            Self::Circle(s) => s.bounds(),
            Self::Line(s) => s.bounds(),
            Self::Ellipse(s) => s.bounds(),
            Self::Path(s) => s.bounds(),
        }
    }

    /// Flips the shape vertically about `center_y` (SVG is Y-down, the designer is
    /// Y-up) and applies the import offset. A flip reverses the sense of rotation.
    fn convert(self, center_y: f64, offset_x: f64, offset_y: f64) -> Shape {
        let flip = |p: Point| Point::new(p.x + offset_x, -p.y + 2.0 * center_y + offset_y);
        match self {
            Self::Rect(r) => {
                let c = flip(r.center);
                let mut rect =
                    Rectangle::new(c.x - r.width / 2.0, c.y - r.height / 2.0, r.width, r.height);
                rect.rotation = -r.rotation;
                Shape::Rectangle(rect)
            }
            Self::Circle(c) => Shape::Circle(Circle::new(flip(c.center), c.radius)),
            Self::Line(l) => Shape::Line(Line::new(flip(l.start), flip(l.end))),
            Self::Ellipse(e) => {
                let mut ellipse = Ellipse::new(flip(e.center), e.rx, e.ry);
                ellipse.rotation = -e.rotation;
                Shape::Ellipse(ellipse)
            }
            Self::Path(p) => {
                // y' = -y + 2c + off_y, x' = x + off_x
                let transform = lyon::math::Transform::new(
                    1.0,
                    0.0,
                    0.0,
                    -1.0,
                    offset_x as f32,
                    (2.0 * center_y + offset_y) as f32,
                );
                let mut new_p = p;
                new_p.transform(&transform);
                Shape::Path(new_p)
            }
        }
    }
}

/// SVG container elements whose children are never rendered directly.
const SVG_NON_RENDERED: &[&str] = &[
    "defs",
    "clipPath",
    "mask",
    "symbol",
    "pattern",
    "marker",
    "linearGradient",
    "radialGradient",
    "style",
    "title",
    "desc",
    "metadata",
];

/// Builds a closed lyon path through `pts`.
fn closed_polygon_path(pts: &[Point]) -> Path {
    let mut b = Path::builder();
    if let Some((first, rest)) = pts.split_first() {
        b.begin(point(first.x as f32, first.y as f32));
        for p in rest {
            b.line_to(point(p.x as f32, p.y as f32));
        }
        b.close();
    }
    b.build()
}

/// Builds an ellipse outline (four cubic arcs) in local coordinates.
fn ellipse_path(cx: f64, cy: f64, rx: f64, ry: f64) -> Path {
    // Cubic Bezier circle constant.
    const K: f64 = 0.552_284_749_830_793_4;
    let p = |x: f64, y: f64| point(x as f32, y as f32);
    let mut b = Path::builder();
    b.begin(p(cx + rx, cy));
    b.cubic_bezier_to(
        p(cx + rx, cy + K * ry),
        p(cx + K * rx, cy + ry),
        p(cx, cy + ry),
    );
    b.cubic_bezier_to(
        p(cx - K * rx, cy + ry),
        p(cx - rx, cy + K * ry),
        p(cx - rx, cy),
    );
    b.cubic_bezier_to(
        p(cx - rx, cy - K * ry),
        p(cx - K * rx, cy - ry),
        p(cx, cy - ry),
    );
    b.cubic_bezier_to(
        p(cx + K * rx, cy - ry),
        p(cx + rx, cy - K * ry),
        p(cx + rx, cy),
    );
    b.close();
    b.build()
}

/// Wraps a lyon path transformed by `ctm` as an imported path shape.
fn transformed_path(path: &Path, ctm: &svg_parse::Affine) -> ImportedShape {
    ImportedShape::Path(PathShape::from_lyon_path(
        &path.clone().transformed(&ctm.to_lyon()),
    ))
}

impl SvgImporter {
    /// Create a new SVG importer with optional scaling
    pub fn new(scale: f64, offset_x: f64, offset_y: f64) -> Self {
        Self {
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Imports SVG from string content.
    ///
    /// Geometry from `path`, `rect`, `circle`, `ellipse`, `line`, `polyline` and
    /// `polygon` elements is converted to millimetres using the full transform stack:
    /// the root `viewBox` → physical size mapping (`width`/`height` in mm, cm, in, pt,
    /// pc or px at 96 dpi; unitless values are taken as mm), every enclosing group's
    /// and the element's own `transform` attribute (`matrix`, `translate`, `scale`,
    /// `rotate`, `skewX`, `skewY`), and finally the importer's `scale`. Content of
    /// non-rendered containers such as `<defs>` is skipped.
    ///
    /// # Arguments
    /// * `svg_content` - The SVG document text.
    ///
    /// # Returns
    /// The imported design (shapes flipped to the designer's Y-up axis), or an error
    /// when the text has no `<svg>` element.
    pub fn import_string(&self, svg_content: &str) -> Result<ImportedDesign> {
        use svg_parse::{Affine, TagKind};

        let tags = svg_parse::tokenize(svg_content);
        let Some(root_idx) = tags.iter().position(|t| t.name == "svg") else {
            anyhow::bail!("Invalid SVG: missing <svg> element");
        };

        let user_scale = Affine::scale(self.scale, self.scale);
        let (root_t, (doc_w, doc_h)) = svg_parse::root_transform(&tags[root_idx]);
        let root_ctm = user_scale.mul(&root_t);

        let mut imported_shapes: Vec<ImportedShape> = Vec::new();
        // Current transformation matrix of each open element.
        let mut stack: Vec<(String, Affine)> = Vec::new();
        // Depth of the stack at which a non-rendered container was opened.
        let mut hidden_from: Option<usize> = None;

        for tag in &tags[root_idx..] {
            if tag.kind == TagKind::End {
                if let Some(idx) = stack.iter().rposition(|(n, _)| *n == tag.name) {
                    stack.truncate(idx);
                    if hidden_from.is_some_and(|h| stack.len() <= h) {
                        hidden_from = None;
                    }
                }
                continue;
            }

            let parent_ctm = stack.last().map(|(_, m)| *m).unwrap_or(root_ctm);
            let ctm = if stack.is_empty() && tag.name == "svg" {
                // The root's own transform attribute is applied inside its viewBox.
                root_ctm.mul(
                    &tag.attr("transform")
                        .map(svg_parse::parse_transform)
                        .unwrap_or(Affine::IDENTITY),
                )
            } else {
                parent_ctm.mul(
                    &tag.attr("transform")
                        .map(svg_parse::parse_transform)
                        .unwrap_or(Affine::IDENTITY),
                )
            };

            if hidden_from.is_none() && SVG_NON_RENDERED.contains(&tag.name.as_str()) {
                hidden_from = Some(stack.len());
            }
            if hidden_from.is_none() {
                if let Some(shape) = Self::element_to_shape(tag, &ctm) {
                    imported_shapes.push(shape);
                }
            }

            if tag.kind == TagKind::Start {
                stack.push((tag.name.clone(), ctm));
            } else if hidden_from.is_some_and(|h| h == stack.len())
                && SVG_NON_RENDERED.contains(&tag.name.as_str())
            {
                // A self-closing non-rendered element hides nothing after it.
                hidden_from = None;
            }
        }

        // Calculate bounds and mirror
        let mut min_y = f64::MAX;
        let mut max_y = f64::MIN;

        for shape in &imported_shapes {
            let (_, s_min_y, _, s_max_y) = shape.bounds();
            if s_min_y < min_y {
                min_y = s_min_y;
            }
            if s_max_y > max_y {
                max_y = s_max_y;
            }
        }

        let center_y = if min_y == f64::MAX {
            0.0
        } else {
            (min_y + max_y) / 2.0
        };

        let shapes: Vec<Shape> = imported_shapes
            .into_iter()
            .map(|s| s.convert(center_y, self.offset_x, self.offset_y))
            .collect();

        // Default to 0 layers for SVG import unless more explicit layer logic is implemented
        let layer_count = 0usize;

        Ok(ImportedDesign {
            shapes,
            dimensions: (doc_w * self.scale, doc_h * self.scale),
            format: FileFormat::Svg,
            layer_count,
            mesh_3d: None,
            unsupported_entities: Vec::new(),
            replaced_numbers: 0,
            z_metadata: Vec::new(),
        })
    }

    /// Converts one SVG geometry element, already resolved to the transform `ctm`
    /// (element user units → mm), into an imported shape.
    ///
    /// Primitives keep their type when `ctm` allows it (axis-aligned scale, or a
    /// rotation with uniform scale for rectangles and ellipses); otherwise they become
    /// transformed paths so skews and non-uniform scales are exact.
    fn element_to_shape(tag: &svg_parse::XmlTag, ctm: &svg_parse::Affine) -> Option<ImportedShape> {
        let num = |name: &str| tag.attr_f64(name).unwrap_or(0.0);
        let points_attr = || -> Vec<Point> {
            tag.attr("points")
                .map(svg_parse::parse_numbers)
                .unwrap_or_default()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| Point::new(c[0], c[1]))
                .collect()
        };

        match tag.name.as_str() {
            "rect" => {
                let (x, y, w, h) = (num("x"), num("y"), num("width"), num("height"));
                if w <= 0.0 || h <= 0.0 {
                    return None;
                }
                let center = ctm.apply(Point::new(x + w / 2.0, y + h / 2.0));
                if ctm.is_axis_aligned() {
                    let (sw, sh) = (w * ctm.a.abs(), h * ctm.d.abs());
                    let rect = Rectangle::new(center.x - sw / 2.0, center.y - sh / 2.0, sw, sh);
                    Some(ImportedShape::Rect(rect))
                } else if let Some((s, angle)) = ctm.as_similarity() {
                    let mut rect = Rectangle::new(
                        center.x - w * s / 2.0,
                        center.y - h * s / 2.0,
                        w * s,
                        h * s,
                    );
                    rect.rotation = angle;
                    Some(ImportedShape::Rect(rect))
                } else {
                    let corners = [
                        Point::new(x, y),
                        Point::new(x + w, y),
                        Point::new(x + w, y + h),
                        Point::new(x, y + h),
                    ];
                    Some(transformed_path(&closed_polygon_path(&corners), ctm))
                }
            }
            "circle" | "ellipse" => {
                let (cx, cy) = (num("cx"), num("cy"));
                let (rx, ry) = if tag.name == "circle" {
                    (num("r"), num("r"))
                } else {
                    (num("rx"), num("ry"))
                };
                if rx <= 0.0 || ry <= 0.0 {
                    return None;
                }
                let center = ctm.apply(Point::new(cx, cy));
                if let Some((s, angle)) = ctm.as_similarity() {
                    if (rx - ry).abs() < 1e-12 {
                        return Some(ImportedShape::Circle(Circle::new(center, rx * s)));
                    }
                    let mut e = Ellipse::new(center, rx * s, ry * s);
                    e.rotation = angle;
                    Some(ImportedShape::Ellipse(e))
                } else if ctm.is_axis_aligned() {
                    Some(ImportedShape::Ellipse(Ellipse::new(
                        center,
                        rx * ctm.a.abs(),
                        ry * ctm.d.abs(),
                    )))
                } else {
                    Some(transformed_path(&ellipse_path(cx, cy, rx, ry), ctm))
                }
            }
            "line" => Some(ImportedShape::Line(Line::new(
                ctm.apply(Point::new(num("x1"), num("y1"))),
                ctm.apply(Point::new(num("x2"), num("y2"))),
            ))),
            "polyline" | "polygon" => {
                let pts: Vec<Point> = points_attr().into_iter().map(|p| ctm.apply(p)).collect();
                if pts.is_empty() {
                    return None;
                }
                Some(ImportedShape::Path(PathShape::from_points(
                    &pts,
                    tag.name == "polygon",
                )))
            }
            "path" => {
                let d = tag.attr("d")?;
                let mut path = PathShape::from_svg_path(d)?;
                path.transform(&ctm.to_lyon());
                Some(ImportedShape::Path(path))
            }
            _ => None,
        }
    }
}

/// Shapes converted from DXF entities, paired with their Z metadata.
type ConvertedEntities = (Vec<Shape>, Vec<(f64, f64)>);

/// DXF importer for converting DXF files to Designer shapes
pub struct DxfImporter {
    pub scale: f64,
    pub offset_x: f64,
    pub offset_y: f64,
}

impl DxfImporter {
    /// Create a new DXF importer with optional scaling
    pub fn new(scale: f64, offset_x: f64, offset_y: f64) -> Self {
        Self {
            scale,
            offset_x,
            offset_y,
        }
    }

    /// Import DXF from file path
    ///
    /// # Arguments
    /// * `path` - Path to DXF file
    ///
    /// # Returns
    /// Imported design with converted shapes
    pub fn import_file(&self, path: &str) -> Result<ImportedDesign> {
        let content =
            std::fs::read_to_string(path).map_err(|e| anyhow!("Failed to read DXF file: {}", e))?;

        self.import_string(&content)
    }

    /// Import DXF from string content
    ///
    /// # Arguments
    /// * `content` - DXF file content as string
    ///
    /// # Returns
    /// Imported design with converted shapes
    pub fn import_string(&self, content: &str) -> Result<ImportedDesign> {
        let mut dxf_file = DxfParser::parse(content)?;

        // Convert the drawing units ($INSUNITS) to millimetres, then apply the
        // caller's scale.
        let unit_factor = dxf_file.header.unit.to_mm_factor();
        dxf_file.scale(unit_factor * self.scale);

        // Convert DXF entities to Designer shapes
        let (shapes, z_metadata) = self.convert_entities_to_shapes(&dxf_file)?;

        // Calculate dimensions from bounding box
        let (min, max) = dxf_file.bounds();
        let dimensions = ((max.x - min.x).abs(), (max.y - min.y).abs());

        let mut unsupported_entities: Vec<(String, usize)> = dxf_file
            .unsupported_entities
            .iter()
            .map(|(name, count)| (name.clone(), *count))
            .collect();
        unsupported_entities.sort();

        Ok(ImportedDesign {
            shapes,
            dimensions,
            format: FileFormat::Dxf,
            layer_count: dxf_file.layer_names().len(),
            mesh_3d: None,
            unsupported_entities,
            replaced_numbers: dxf_file.replaced_numbers,
            z_metadata,
        })
    }

    /// Convert DXF entities to Designer shapes
    ///
    /// Note: DXF coordinates are negated on X-axis to correct for coordinate system difference.
    /// DXF uses right-handed coordinate system, Designer uses left-handed with Y-up.
    fn convert_entities_to_shapes(&self, dxf_file: &DxfFile) -> Result<ConvertedEntities> {
        let mut shapes: Vec<Shape> = Vec::new();
        let mut z_metadata: Vec<(f64, f64)> = Vec::new();

        // Transform to apply: negate X and add offset
        // Note: dxf_file is already scaled by self.scale
        //        let transform = lyon::math::Transform::scale(-1.0, 1.0).then_translate(lyon::math::vector(
        let transform =
            lyon::math::Transform::translation(self.offset_x as f32, self.offset_y as f32);

        for entity in &dxf_file.entities {
            let entity_z = match entity {
                DxfEntity::Line(l) => (l.elevation, l.thickness),
                DxfEntity::Circle(c) => (c.elevation, c.thickness),
                DxfEntity::Arc(a) => (a.elevation, a.thickness),
                DxfEntity::Ellipse(e) => (e.elevation, e.thickness),
                DxfEntity::Polyline(p) => (p.elevation, p.thickness),
                DxfEntity::Text(_) => (0.0, 0.0),
            };

            let path_opt = match entity {
                DxfEntity::Line(line) => {
                    let mut builder = Path::builder();
                    builder.begin(point(line.start.x as f32, line.start.y as f32));
                    builder.line_to(point(line.end.x as f32, line.end.y as f32));
                    builder.end(false);
                    Some(builder.build())
                }

                DxfEntity::Circle(circle) => {
                    let mut builder = Path::builder();
                    let center = point(circle.center.x as f32, circle.center.y as f32);
                    let radius = circle.radius as f32;

                    let steps = 64; // Número de puntos para un círculo suave
                    let start_point = center + lyon::math::vector(radius, 0.0);
                    builder.begin(start_point);

                    for i in 1..=steps {
                        let angle = 2.0 * std::f32::consts::PI * (i as f32 / steps as f32);
                        let x = center.x + radius * angle.cos();
                        let y = center.y + radius * angle.sin();
                        builder.line_to(point(x, y));
                    }

                    builder.close();
                    Some(builder.build())
                }

                DxfEntity::Arc(arc) => {
                    let mut builder = Path::builder();
                    let center = point(arc.center.x as f32, arc.center.y as f32);
                    let radius = arc.radius as f32;

                    let start_angle = arc.start_angle.to_radians() as f32;
                    let end_angle = arc.end_angle.to_radians() as f32;

                    let mut sweep_angle = end_angle - start_angle;
                    if sweep_angle < 0.0 {
                        sweep_angle += 2.0 * std::f32::consts::PI;
                    }

                    let steps = 64; // Número de puntos para un arco suave
                    let steps =
                        (steps as f32 * sweep_angle / (2.0 * std::f32::consts::PI)).ceil() as usize;
                    let steps = steps.max(4); // Mínimo 4 puntos para arcos pequeños

                    // Punto inicial
                    let start_point =
                        center + vector(radius * start_angle.cos(), radius * start_angle.sin());
                    builder.begin(start_point);

                    // Generar puntos intermedios
                    for i in 1..=steps {
                        let t = start_angle + sweep_angle * (i as f32 / steps as f32);
                        let x = center.x + radius * t.cos();
                        let y = center.y + radius * t.sin();
                        builder.line_to(point(x, y));
                    }

                    builder.end(false);
                    Some(builder.build())
                }

                DxfEntity::Ellipse(ellipse) => {
                    let center = point(ellipse.center.x as f32, ellipse.center.y as f32);

                    let major_vec =
                        vector(ellipse.major_axis.x as f32, ellipse.major_axis.y as f32);
                    let major_radius =
                        (major_vec.x * major_vec.x + major_vec.y * major_vec.y).sqrt();
                    let minor_radius = major_radius * ellipse.ratio as f32;

                    let rotation = major_vec.y.atan2(major_vec.x);

                    let start_angle = ellipse.start_angle as f32;
                    let end_angle = ellipse.end_angle as f32;

                    // Calc sweep angle
                    let mut sweep_angle = end_angle - start_angle;
                    if sweep_angle < 0.0 {
                        sweep_angle += 2.0 * std::f32::consts::PI;
                    }

                    let steps = 64;
                    let mut builder = Path::builder();

                    let start_x = center.x + major_radius * rotation.cos() * start_angle.cos()
                        - minor_radius * rotation.sin() * start_angle.sin();
                    let start_y = center.y
                        + major_radius * rotation.sin() * start_angle.cos()
                        + minor_radius * rotation.cos() * start_angle.sin();
                    builder.begin(point(start_x, start_y));

                    for i in 1..=steps {
                        let t = start_angle + sweep_angle * (i as f32 / steps as f32);
                        let x = center.x + major_radius * rotation.cos() * t.cos()
                            - minor_radius * rotation.sin() * t.sin();
                        let y = center.y
                            + major_radius * rotation.sin() * t.cos()
                            + minor_radius * rotation.cos() * t.sin();
                        builder.line_to(point(x, y));
                    }

                    let is_complete = (sweep_angle - 2.0 * std::f32::consts::PI).abs() < 0.001;
                    if is_complete {
                        builder.line_to(point(start_x, start_y));
                    }

                    builder.end(false);
                    let path = builder.build();

                    Some(path)
                }

                DxfEntity::Polyline(polyline) => {
                    if polyline.vertices.is_empty() {
                        None
                    } else {
                        use lyon::math::{point, vector};

                        let mut builder = Path::builder();

                        // Start at the first vertex
                        let start = polyline.vertices[0];
                        builder.begin(point(start.x as f32, start.y as f32));

                        // Emits one polyline segment (straight or bulged arc) ending at v2.
                        let mut add_segment = |v1: Point, v2: Point, bulge: f64| {
                            let p1 = point(v1.x as f32, v1.y as f32);
                            let p2 = point(v2.x as f32, v2.y as f32);

                            let dist =
                                ((v2.x - v1.x).powi(2) + (v2.y - v1.y).powi(2)).sqrt() as f32;

                            if bulge.abs() < 0.0001 || dist < 1e-9 {
                                // Straight line
                                builder.line_to(p2);
                            } else {
                                let included_angle = 4.0 * bulge.abs().atan() as f32;

                                let h = bulge.abs() as f32 * (dist / 2.0);

                                let radius = ((dist / 2.0).powi(2) + h.powi(2)) / (2.0 * h);

                                let dist_to_center = radius - h;

                                let mid =
                                    point((v1.x + v2.x) as f32 / 2.0, (v1.y + v2.y) as f32 / 2.0);

                                let perp = vector(
                                    -(v2.y - v1.y) as f32 / dist,
                                    (v2.x - v1.x) as f32 / dist,
                                );

                                let center = if bulge > 0.0 {
                                    mid + perp * dist_to_center
                                } else {
                                    mid - perp * dist_to_center
                                };

                                let start_vec = p1 - center;
                                let start_angle = start_vec.y.atan2(start_vec.x);

                                let sweep_angle = if bulge > 0.0 {
                                    included_angle
                                } else {
                                    -included_angle
                                };

                                let arc_geom = lyon::geom::Arc {
                                    center,
                                    radii: vector(radius, radius),
                                    x_rotation: lyon::math::Angle::radians(0.0),
                                    start_angle: lyon::math::Angle::radians(start_angle),
                                    sweep_angle: lyon::math::Angle::radians(sweep_angle),
                                };

                                arc_geom.for_each_cubic_bezier(&mut |ctrl| {
                                    builder.cubic_bezier_to(ctrl.ctrl1, ctrl.ctrl2, ctrl.to);
                                });
                            }
                        };

                        // Bulge for the segment starting at vertex i (0 = straight line)
                        let bulge_at = |i: usize| polyline.bulges.get(i).copied().unwrap_or(0.0);

                        // Process each segment with its potential bulge
                        let n = polyline.vertices.len();
                        for i in 0..n - 1 {
                            add_segment(
                                polyline.vertices[i],
                                polyline.vertices[i + 1],
                                bulge_at(i),
                            );
                        }

                        // A closed polyline's last vertex carries the bulge of the closing
                        // segment back to the first vertex; without this it would be drawn
                        // as a straight line (e.g. rounded ends of slots).
                        if polyline.closed && n > 1 && bulge_at(n - 1).abs() >= 0.0001 {
                            add_segment(
                                polyline.vertices[n - 1],
                                polyline.vertices[0],
                                bulge_at(n - 1),
                            );
                        }

                        if polyline.closed {
                            builder.close();
                        } else {
                            builder.end(false);
                        }
                        Some(builder.build())
                    }
                }
                _ => None,
            };

            if let Some(path) = path_opt {
                let mut shape = PathShape::from_lyon_path(&path);
                shape.transform(&transform);
                shapes.push(Shape::Path(shape));
                z_metadata.push(entity_z);
            }
        }

        Ok((shapes, z_metadata))
    }
}

/// STL importer for converting 3D STL files to Designer shapes via shadow projection
pub struct StlImporter {
    pub scale: f32,
    pub center_model: bool,
    pub projection_direction: nalgebra::Vector3<f32>,
}

impl StlImporter {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            center_model: true,
            projection_direction: nalgebra::Vector3::new(0.0, 0.0, -1.0), // Project along Z-axis
        }
    }

    pub fn with_scale(mut self, scale: f32) -> Self {
        self.scale = scale;
        self
    }

    pub fn with_centering(mut self, center: bool) -> Self {
        self.center_model = center;
        self
    }

    pub fn with_projection_direction(mut self, direction: nalgebra::Vector3<f32>) -> Self {
        self.projection_direction = direction.normalize();
        self
    }

    /// Import STL file and return both 3D mesh and 2D shadow projection
    pub fn import_file(&self, path: &str) -> Result<ImportedDesign> {
        let importer = Model3DImporter::new()
            .with_scale(self.scale)
            .with_centering(self.center_model);

        let mesh = importer.import_file(path)?;
        self.create_imported_design(mesh)
    }

    /// Import STL from binary data
    pub fn import_data(&self, data: &[u8]) -> Result<ImportedDesign> {
        let importer = Model3DImporter::new()
            .with_scale(self.scale)
            .with_centering(self.center_model);

        let mesh = importer.import_stl_data(data)?;
        self.create_imported_design(mesh)
    }

    /// Project 3D mesh to 2D shadow and create ImportedDesign
    fn create_imported_design(&self, mesh: Mesh3D) -> Result<ImportedDesign> {
        // Generate 2D shadow projection
        let shapes = mesh.project_shadow_z()?;

        // Calculate 2D dimensions from mesh bounds
        let width = (mesh.bounds_max.x - mesh.bounds_min.x) as f64;
        let height = (mesh.bounds_max.y - mesh.bounds_min.y) as f64;
        let dimensions = (width, height);

        Ok(ImportedDesign {
            shapes,
            dimensions,
            format: FileFormat::Stl,
            layer_count: 1, // STL shadow projection creates a single layer
            mesh_3d: Some(mesh),
            unsupported_entities: Vec::new(),
            replaced_numbers: 0,
            z_metadata: Vec::new(),
        })
    }

    /// Import STL and slice at specific Z height
    pub fn import_with_slice(&self, path: &str, z_height: f32) -> Result<ImportedDesign> {
        let importer = Model3DImporter::new()
            .with_scale(self.scale)
            .with_centering(self.center_model);

        let mesh = importer.import_file(path)?;

        // Generate 2D slice instead of shadow projection
        let shapes = mesh.slice_at_z(z_height)?;

        // Calculate 2D dimensions from mesh bounds
        let width = (mesh.bounds_max.x - mesh.bounds_min.x) as f64;
        let height = (mesh.bounds_max.y - mesh.bounds_min.y) as f64;
        let dimensions = (width, height);

        Ok(ImportedDesign {
            shapes,
            dimensions,
            format: FileFormat::Stl,
            layer_count: 1, // Single slice creates one layer
            mesh_3d: Some(mesh),
            unsupported_entities: Vec::new(),
            replaced_numbers: 0,
            z_metadata: Vec::new(),
        })
    }
}

impl Default for StlImporter {
    fn default() -> Self {
        Self::new()
    }
}
