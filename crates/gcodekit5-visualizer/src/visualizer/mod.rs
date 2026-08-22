//! 3D visualization module (wgpu-based)
//!
//! This module provides:
//! - 3D rendering engine (setup)
//! - Toolpath visualization (rendering)
//! - Interactive camera controls (controls)
//! - Grid and axis rendering
//! - 3D mesh rendering

pub mod camera;
pub mod controls;
pub mod features;

pub mod setup;
pub mod stock_removal_3d;
pub mod toolpath_cache;
pub mod toolpath_rendering;
pub mod viewport;

#[allow(clippy::module_inception)]
pub mod visualizer;

pub use camera::Camera as Camera3D;

pub use controls::{CameraController, ViewPreset, VisualizerControls};
pub use features::{
    BoundingBox, GridConfig, MachineLimits, SceneFeatures, ToolMarker, WorkCoordinateSystem,
};

pub use setup::{Camera, CameraType, Color, Light, LightType, Renderer, Scene, Vector3};
pub use stock_removal_3d::{
    generate_surface_mesh, StockSimulator3D, ToolpathSegment, ToolpathSegmentType, VoxelGrid,
};
pub use toolpath_cache::ToolpathCache;
pub use toolpath_rendering::{
    ArcSegment, LineSegment, MovementType, PathSegment, Toolpath, ToolpathStats,
};
pub use viewport::{Bounds};
pub use visualizer::{GCodeCommand, Point3D, Visualizer};
