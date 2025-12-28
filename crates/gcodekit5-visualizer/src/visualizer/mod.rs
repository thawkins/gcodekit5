//! 3D visualization module (wgpu-based)
//!
//! This module provides:
//! - 3D rendering engine (setup)
//! - Toolpath visualization (rendering)
//! - Interactive camera controls (controls)
//! - Grid and axis rendering
//! - 3D mesh rendering for STL models

pub mod camera;
pub mod canvas_renderer;
pub mod controls;
pub mod features;
pub mod mesh_rendering;
pub mod mesh_renderer;
pub mod mesh_shaders;
pub mod scene3d;
pub mod setup;
pub mod stock_removal_3d;
pub mod toolpath_cache;
pub mod toolpath_rendering;
pub mod viewport;
pub mod visualizer;

pub use camera::Camera as Camera3D;
pub use canvas_renderer::{
    render_g1_to_path, render_g2_to_path, render_g3_to_path, render_g4_to_path,
    render_grid_to_path, render_intensity_overlay, render_origin_to_path,
    render_rapid_moves_to_path, render_toolpath_to_path,
};
pub use controls::{CameraController, ViewPreset, VisualizerControls};
pub use features::{
    BoundingBox, GridConfig, MachineLimits, SceneFeatures, ToolMarker, WorkCoordinateSystem,
};
pub use mesh_rendering::{RenderableMesh, MeshCollection, MeshMaterial};
pub use mesh_renderer::{MeshRenderer, LightingParams, MeshRenderError};
pub use scene3d::{Scene3D, Scene3DStats, Renderer3D, stl_integration};
pub use setup::{Camera, CameraType, Color, Light, LightType, Renderer, Scene, Vector3};
pub use stock_removal_3d::{
    generate_surface_mesh, StockSimulator3D, ToolpathSegment, ToolpathSegmentType, VoxelGrid,
};
pub use toolpath_cache::ToolpathCache;
pub use toolpath_rendering::{
    ArcSegment, LineSegment, MovementType, PathSegment, Toolpath, ToolpathStats,
};
pub use viewport::{Bounds, ViewportTransform};
pub use visualizer::{GCodeCommand, Point3D, Visualizer};

// Removed conflicting Visualizer struct definition as it is now imported from visualizer module
