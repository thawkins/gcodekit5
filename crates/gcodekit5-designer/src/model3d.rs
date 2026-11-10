//! # 3D Model Module
//!
//! Provides data structures and functionality for importing and working with 3D models.
//! Supports STL mesh format and shadow projection to 2D shapes.
//!
//! ## Supported Formats
//! - STL (STereoLithography) - Triangle mesh format
//!
//! ## Features
//! - 3D model import from STL files
//! - Shadow projection to generate 2D silhouettes
//! - Model slicing for 2.5D machining
//! - Coordinate transformation and scaling

use crate::model::Point;
use crate::model::{DesignPath as PathShape, Shape};
use anyhow::{Result, anyhow};
use csgrs::sketch::Sketch;
use csgrs::traits::CSG;
use nalgebra::{Matrix4, Point3, Vector3};
use std::collections::HashMap;
use tracing::debug;

/// A 3D triangle made up of three vertices
#[derive(Debug, Clone, PartialEq)]
pub struct Triangle3D {
    pub vertices: [Point3<f32>; 3],
    pub normal: Vector3<f32>,
}

impl Triangle3D {
    pub fn new(v1: Point3<f32>, v2: Point3<f32>, v3: Point3<f32>) -> Self {
        // Calculate normal using cross product
        let edge1 = v2 - v1;
        let edge2 = v3 - v1;
        // A degenerate (zero-area) triangle has no direction; use zero rather
        // than the NaN that normalising a zero vector gives.
        let normal = edge1
            .cross(&edge2)
            .try_normalize(f32::EPSILON)
            .unwrap_or_else(Vector3::zeros);

        Self {
            vertices: [v1, v2, v3],
            normal,
        }
    }

    /// Get bounding box of the triangle
    pub fn bounds(&self) -> (Point3<f32>, Point3<f32>) {
        let mut min_x = self.vertices[0].x;
        let mut max_x = self.vertices[0].x;
        let mut min_y = self.vertices[0].y;
        let mut max_y = self.vertices[0].y;
        let mut min_z = self.vertices[0].z;
        let mut max_z = self.vertices[0].z;

        for vertex in &self.vertices[1..] {
            min_x = min_x.min(vertex.x);
            max_x = max_x.max(vertex.x);
            min_y = min_y.min(vertex.y);
            max_y = max_y.max(vertex.y);
            min_z = min_z.min(vertex.z);
            max_z = max_z.max(vertex.z);
        }

        (
            Point3::new(min_x, min_y, min_z),
            Point3::new(max_x, max_y, max_z),
        )
    }

    /// Check if triangle intersects with a horizontal plane at given Z height
    pub fn intersects_plane_z(&self, z: f32) -> bool {
        let z_min = self.vertices[0]
            .z
            .min(self.vertices[1].z)
            .min(self.vertices[2].z);
        let z_max = self.vertices[0]
            .z
            .max(self.vertices[1].z)
            .max(self.vertices[2].z);
        z >= z_min && z <= z_max
    }

    /// Get intersection points with horizontal plane at Z height
    pub fn intersect_plane_z(&self, z: f32) -> Vec<Point3<f32>> {
        if !self.intersects_plane_z(z) {
            return vec![];
        }

        let mut intersections = Vec::new();

        // Check each edge for intersection with Z plane
        for i in 0..3 {
            let v1 = self.vertices[i];
            let v2 = self.vertices[(i + 1) % 3];

            if let Some(intersection) = intersect_edge_with_plane_z(v1, v2, z) {
                intersections.push(intersection);
            }
        }

        // A vertex on the plane is found by both edges that share it, which are
        // not always next to each other in the list: remove every duplicate.
        let mut unique: Vec<Point3<f32>> = Vec::with_capacity(intersections.len());
        for p in intersections {
            if !unique
                .iter()
                .any(|q| (p.x - q.x).abs() < 1e-6 && (p.y - q.y).abs() < 1e-6)
            {
                unique.push(p);
            }
        }
        unique
    }
}

/// Intersect an edge with a horizontal plane at Z height
fn intersect_edge_with_plane_z(v1: Point3<f32>, v2: Point3<f32>, z: f32) -> Option<Point3<f32>> {
    // If both vertices are on the same side of the plane, no intersection
    if (v1.z - z) * (v2.z - z) > 0.0 {
        return None;
    }

    // If one vertex is exactly on the plane, return it
    if (v1.z - z).abs() < 1e-6 {
        return Some(v1);
    }
    if (v2.z - z).abs() < 1e-6 {
        return Some(v2);
    }

    // Calculate intersection point using linear interpolation
    let t = (z - v1.z) / (v2.z - v1.z);
    let x = v1.x + t * (v2.x - v1.x);
    let y = v1.y + t * (v2.y - v1.y);

    Some(Point3::new(x, y, z))
}

/// A 3D mesh model
#[derive(Debug, Clone)]
pub struct Mesh3D {
    pub triangles: Vec<Triangle3D>,
    pub bounds_min: Point3<f32>,
    pub bounds_max: Point3<f32>,
}

impl Mesh3D {
    pub fn new(triangles: Vec<Triangle3D>) -> Self {
        let mut mesh = Self {
            triangles,
            bounds_min: Point3::new(0.0, 0.0, 0.0),
            bounds_max: Point3::new(0.0, 0.0, 0.0),
        };
        mesh.calculate_bounds();
        mesh
    }

    pub fn from_stl_mesh(stl_mesh: &stl_io::IndexedMesh) -> Self {
        let mut triangles = Vec::new();

        // Convert indexed mesh to triangles
        for face in &stl_mesh.faces {
            let v1_idx = face.vertices[0];
            let v2_idx = face.vertices[1];
            let v3_idx = face.vertices[2];

            if v1_idx < stl_mesh.vertices.len()
                && v2_idx < stl_mesh.vertices.len()
                && v3_idx < stl_mesh.vertices.len()
            {
                let v1 = stl_mesh.vertices[v1_idx];
                let v2 = stl_mesh.vertices[v2_idx];
                let v3 = stl_mesh.vertices[v3_idx];

                triangles.push(Triangle3D::new(
                    Point3::new(v1[0], v1[1], v1[2]),
                    Point3::new(v2[0], v2[1], v2[2]),
                    Point3::new(v3[0], v3[1], v3[2]),
                ));
            }
        }

        Self::new(triangles)
    }

    pub fn from_stl_triangles(stl_triangles: &[stl_io::Triangle]) -> Self {
        let triangles: Vec<Triangle3D> = stl_triangles
            .iter()
            .map(|tri| {
                Triangle3D::new(
                    Point3::new(tri.vertices[0][0], tri.vertices[0][1], tri.vertices[0][2]),
                    Point3::new(tri.vertices[1][0], tri.vertices[1][1], tri.vertices[1][2]),
                    Point3::new(tri.vertices[2][0], tri.vertices[2][1], tri.vertices[2][2]),
                )
            })
            .collect();

        Self::new(triangles)
    }

    fn calculate_bounds(&mut self) {
        if self.triangles.is_empty() {
            return;
        }

        let first_bounds = self.triangles[0].bounds();
        let mut min = first_bounds.0;
        let mut max = first_bounds.1;

        for triangle in &self.triangles[1..] {
            let (tri_min, tri_max) = triangle.bounds();
            min.x = min.x.min(tri_min.x);
            min.y = min.y.min(tri_min.y);
            min.z = min.z.min(tri_min.z);
            max.x = max.x.max(tri_max.x);
            max.y = max.y.max(tri_max.y);
            max.z = max.z.max(tri_max.z);
        }

        self.bounds_min = min;
        self.bounds_max = max;
    }

    /// Transform the mesh using a 4x4 transformation matrix
    pub fn transform(&mut self, transform: &Matrix4<f32>) {
        for triangle in &mut self.triangles {
            for vertex in &mut triangle.vertices {
                let transformed = transform.transform_point(vertex);
                *vertex = transformed;
            }

            // Recalculate normal after transformation
            let edge1 = triangle.vertices[1] - triangle.vertices[0];
            let edge2 = triangle.vertices[2] - triangle.vertices[0];
            triangle.normal = edge1
                .cross(&edge2)
                .try_normalize(f32::EPSILON)
                .unwrap_or_else(Vector3::zeros);
        }

        self.calculate_bounds();
    }

    /// Scale the mesh uniformly
    pub fn scale(&mut self, factor: f32) {
        let transform = Matrix4::new_scaling(factor);
        self.transform(&transform);
    }

    /// Translate the mesh
    pub fn translate(&mut self, offset: Vector3<f32>) {
        let transform = Matrix4::new_translation(&offset);
        self.transform(&transform);
    }

    /// Center the mesh at the origin
    pub fn center(&mut self) {
        let center = (self.bounds_min + self.bounds_max.coords) * 0.5;
        self.translate(-center.coords);
    }

    /// Get all triangles that intersect with a horizontal plane at Z height
    pub fn get_intersecting_triangles(&self, z: f32) -> Vec<&Triangle3D> {
        self.triangles
            .iter()
            .filter(|tri| tri.intersects_plane_z(z))
            .collect()
    }

    /// Project the mesh to a 2D silhouette by projecting along Z-axis
    /// Returns a collection of 2D paths representing the outline
    pub fn project_shadow_z(&self) -> Result<Vec<Shape>> {
        debug!("Projecting 3D mesh to 2D shadow along Z-axis");

        // The outline of each surface patch facing up (or, for a mesh with
        // none, down) is its boundary: the edges used by exactly one of its
        // triangles. Edges are keyed in 3D, so two parts stacked at different
        // heights keep their own outlines instead of cancelling out.
        let facing_up: Vec<&Triangle3D> = self
            .triangles
            .iter()
            .filter(|t| t.normal.z > 1e-6)
            .collect();
        let facing: Vec<&Triangle3D> = if facing_up.is_empty() {
            self.triangles
                .iter()
                .filter(|t| t.normal.z < -1e-6)
                .collect()
        } else {
            facing_up
        };

        let key = |p: &Point3<f32>| {
            (
                (f64::from(p.x) * 1e6).round() as i64,
                (f64::from(p.y) * 1e6).round() as i64,
                (f64::from(p.z) * 1e6).round() as i64,
            )
        };
        // Triangles that share an edge belong to the same patch.
        let mut patch_of = UnionFind::new(facing.len());
        let mut edge_use: HashMap<EdgeKey3, (Point, Point, usize, usize)> = HashMap::new();
        for (index, triangle) in facing.iter().enumerate() {
            let v = &triangle.vertices;
            for (a, b) in [(&v[0], &v[1]), (&v[1], &v[2]), (&v[2], &v[0])] {
                let (ka, kb) = (key(a), key(b));
                if ka == kb {
                    continue;
                }
                let edge_key = if ka < kb { (ka, kb) } else { (kb, ka) };
                let to_2d = |p: &Point3<f32>| Point::new(f64::from(p.x), f64::from(p.y));
                let entry = edge_use
                    .entry(edge_key)
                    .or_insert((to_2d(a), to_2d(b), 0, index));
                entry.2 += 1;
                patch_of.union(entry.3, index);
            }
        }
        let mut boundary: Vec<(EdgeKey3, (Point, Point), usize)> = edge_use
            .into_iter()
            .filter(|(_, (_, _, count, _))| *count == 1)
            .map(|(k, (a, b, _, triangle))| (k, (a, b), patch_of.find(triangle)))
            .collect();
        // Deterministic order, so the same mesh gives the same shapes.
        boundary.sort_by_key(|(k, _, _)| *k);

        debug!("Collected {} edge segments", boundary.len());

        let mut slot_of_patch: HashMap<usize, usize> = HashMap::new();
        let mut patches: Vec<Vec<(Point, Point)>> = Vec::new();
        for (_, segment, patch) in boundary {
            let slot = *slot_of_patch.entry(patch).or_insert_with(|| {
                patches.push(Vec::new());
                patches.len() - 1
            });
            patches[slot].push(segment);
        }

        if patches.len() <= 1 {
            let segments = patches.pop().unwrap_or_default();
            let contours = build_contours_from_segments(&segments)?;
            debug!("Built {} contour paths", contours.len());
            return Ok(contours.into_iter().map(Shape::Path).collect());
        }

        // Several patches can overlap in XY (stacked or overlapping parts):
        // the shadow is the union of their areas. A patch's area is the
        // even-odd fill of its loops, so a hole stays a hole.
        let mut shadow: Sketch<()> = Sketch::new();
        for segments in &patches {
            let mut area: Sketch<()> = Sketch::new();
            for (points, closed) in chain_segments(segments) {
                if closed && points.len() >= 3 {
                    let ring: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
                    area = area.xor(&Sketch::polygon(&ring, None));
                }
            }
            shadow = shadow.union(&area);
        }
        debug!("Merged {} surface patches into one shadow", patches.len());
        let mut path = PathShape::from_csg(shadow);
        path.closed = true;
        Ok(vec![Shape::Path(path)])
    }

    /// Slice the mesh at a specific Z height and return 2D contour paths
    pub fn slice_at_z(&self, z: f32) -> Result<Vec<Shape>> {
        debug!("Slicing mesh at Z = {}", z);

        let mut intersection_segments = Vec::new();

        for triangle in &self.triangles {
            let intersections = triangle.intersect_plane_z(z);
            if intersections.len() == 2 {
                // Two intersection points form a line segment
                let p1 = Point::new(intersections[0].x as f64, intersections[0].y as f64);
                let p2 = Point::new(intersections[1].x as f64, intersections[1].y as f64);
                intersection_segments.push((p1, p2));
            }
        }

        debug!(
            "Found {} intersection segments at Z = {}",
            intersection_segments.len(),
            z
        );

        // Build contour paths from intersection segments
        let contours = build_contours_from_segments(&intersection_segments)?;

        debug!("Built {} contour paths from slice", contours.len());

        let shapes: Vec<Shape> = contours.into_iter().map(Shape::Path).collect();

        Ok(shapes)
    }
}

/// An undirected 3D edge between two points quantised to 1e-6 (smaller end first).
type EdgeKey3 = ((i64, i64, i64), (i64, i64, i64));

/// Disjoint sets over `0..n`, used to group triangles into surface patches.
struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, mut i: usize) -> usize {
        while self.parent[i] != i {
            self.parent[i] = self.parent[self.parent[i]];
            i = self.parent[i];
        }
        i
    }

    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[rb] = ra;
        }
    }
}

/// Build contour paths from a collection of line segments
/// This function attempts to connect segments into closed loops
pub fn build_contours_from_segments(segments: &[(Point, Point)]) -> Result<Vec<PathShape>> {
    Ok(chain_segments(segments)
        .into_iter()
        .map(|(points, closed)| PathShape::from_points(&points, closed))
        .collect())
}

/// Chains segments whose endpoints are within 1e-6 into polylines.
///
/// # Returns
/// Each chain's points (at least two) and whether it closes on its start.
fn chain_segments(segments: &[(Point, Point)]) -> Vec<(Vec<Point>, bool)> {
    const TOLERANCE: f64 = 1e-6;
    let near = |a: Point, b: Point| (a.x - b.x).abs() < TOLERANCE && (a.y - b.y).abs() < TOLERANCE;

    // Index endpoints by a 1e-6 grid so each extension is a lookup instead of
    // a scan of every segment (the scan made a 20 000-triangle import take
    // about 26 s). Two points within the tolerance can round into
    // neighbouring cells, so a lookup searches the 3 x 3 cells around it and
    // compares the real coordinates.
    let key = |p: Point| ((p.x * 1e6).round() as i64, (p.y * 1e6).round() as i64);
    let mut by_endpoint: HashMap<(i64, i64), Vec<usize>> = HashMap::new();
    for (index, &(a, b)) in segments.iter().enumerate() {
        by_endpoint.entry(key(a)).or_default().push(index);
        if key(b) != key(a) {
            by_endpoint.entry(key(b)).or_default().push(index);
        }
    }

    let mut used = vec![false; segments.len()];
    let mut chains = Vec::new();

    for (i, &(start, end)) in segments.iter().enumerate() {
        if used[i] {
            continue;
        }

        // Start a new chain and extend it with the first unused segment (in
        // input order) that touches its end.
        let mut points = vec![start];
        let mut current_end = end;
        used[i] = true;

        loop {
            let (cx, cy) = key(current_end);
            let next = (-1..=1)
                .flat_map(|dx| (-1..=1).map(move |dy| (cx + dx, cy + dy)))
                .filter_map(|cell| by_endpoint.get(&cell))
                .flatten()
                .copied()
                .filter(|&j| !used[j])
                .filter(|&j| near(segments[j].0, current_end) || near(segments[j].1, current_end))
                .min();
            let Some(j) = next else { break };
            let (seg_start, seg_end) = segments[j];
            current_end = if near(seg_start, current_end) {
                seg_end
            } else {
                seg_start
            };
            points.push(current_end);
            used[j] = true;
        }

        let closed = points.len() > 2 && near(current_end, start);
        if points.len() > 1 {
            chains.push((points, closed));
        }
    }

    chains
}

/// Supported 3D file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Model3DFormat {
    /// STL (STereoLithography) format
    Stl,
}

/// 3D model importer for converting 3D files to mesh representations
pub struct Model3DImporter {
    pub scale: f32,
    pub center_model: bool,
}

impl Model3DImporter {
    pub fn new() -> Self {
        Self {
            scale: 1.0,
            center_model: true,
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

    /// Import 3D model from file path
    pub fn import_file(&self, path: &str) -> Result<Mesh3D> {
        let format = self.detect_format(path)?;

        match format {
            Model3DFormat::Stl => self.import_stl_file(path),
        }
    }

    /// Import STL from file path
    pub fn import_stl_file(&self, path: &str) -> Result<Mesh3D> {
        debug!("Importing STL file: {}", path);

        let mut file = std::fs::File::open(path)
            .map_err(|e| anyhow!("Failed to open STL file {}: {}", path, e))?;

        let stl = stl_io::read_stl(&mut file)
            .map_err(|e| anyhow!("Failed to parse STL file {}: {}", path, e))?;

        debug!("STL contains {} faces", stl.faces.len());

        let mut mesh = Mesh3D::from_stl_mesh(&stl);

        // Apply transformations
        if self.scale != 1.0 {
            debug!("Scaling mesh by factor {}", self.scale);
            mesh.scale(self.scale);
        }

        if self.center_model {
            debug!("Centering mesh at origin");
            mesh.center();
        }

        debug!(
            "Final mesh bounds: {:?} to {:?}",
            mesh.bounds_min, mesh.bounds_max
        );

        Ok(mesh)
    }

    /// Import STL from string content (binary STL data)
    pub fn import_stl_data(&self, data: &[u8]) -> Result<Mesh3D> {
        debug!("Importing STL from binary data ({} bytes)", data.len());

        let mut cursor = std::io::Cursor::new(data);
        let stl = stl_io::read_stl(&mut cursor)
            .map_err(|e| anyhow!("Failed to parse STL data: {}", e))?;

        debug!("STL contains {} faces", stl.faces.len());

        let mut mesh = Mesh3D::from_stl_mesh(&stl);

        // Apply transformations
        if self.scale != 1.0 {
            debug!("Scaling mesh by factor {}", self.scale);
            mesh.scale(self.scale);
        }

        if self.center_model {
            debug!("Centering mesh at origin");
            mesh.center();
        }

        debug!(
            "Final mesh bounds: {:?} to {:?}",
            mesh.bounds_min, mesh.bounds_max
        );

        Ok(mesh)
    }

    /// Detect file format from file extension
    fn detect_format(&self, path: &str) -> Result<Model3DFormat> {
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|s| s.to_lowercase())
            .ok_or_else(|| anyhow!("Unable to determine file extension for: {}", path))?;

        match extension.as_str() {
            "stl" => Ok(Model3DFormat::Stl),
            _ => Err(anyhow!("Unsupported 3D file format: .{}", extension)),
        }
    }
}

impl Default for Model3DImporter {
    fn default() -> Self {
        Self::new()
    }
}

/// Projection parameters for shadow generation
#[derive(Debug, Clone)]
pub struct ProjectionParams {
    /// Direction vector for projection (default: along negative Z-axis)
    pub direction: Vector3<f32>,
    /// Whether to generate filled silhouette or just outline
    pub filled: bool,
    /// Simplification tolerance for generated paths
    pub tolerance: f64,
}

impl Default for ProjectionParams {
    fn default() -> Self {
        Self {
            direction: Vector3::new(0.0, 0.0, -1.0), // Project along Z-axis
            filled: false,
            tolerance: 0.01,
        }
    }
}
