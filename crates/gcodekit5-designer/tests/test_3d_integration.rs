use gcodekit5_designer::model3d::{Mesh3D, Triangle3D};
use gcodekit5_designer::shadow_projection::ShadowProjector;
use gcodekit5_designer::slice_toolpath::{SliceToToolpath, SliceStrategy};
use gcodekit5_core::units::Point3D;

#[test]
fn test_mesh3d_creation() {
    let triangles = vec![
        Triangle3D::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(1.0, 0.0, 0.0),
            Point3D::new(0.5, 1.0, 0.0),
        ),
        Triangle3D::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(0.5, 1.0, 0.0),
            Point3D::new(0.0, 1.0, 1.0),
        ),
    ];
    
    let mesh = Mesh3D::new(triangles);
    assert_eq!(mesh.triangle_count(), 2);
    
    let bounds = mesh.bounds();
    assert_eq!(bounds.min.x, 0.0);
    assert_eq!(bounds.max.x, 1.0);
    assert_eq!(bounds.min.y, 0.0);
    assert_eq!(bounds.max.y, 1.0);
    assert_eq!(bounds.min.z, 0.0);
    assert_eq!(bounds.max.z, 1.0);
}

#[test] 
fn test_shadow_projection_orthographic() {
    let triangles = vec![
        Triangle3D::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(1.0, 0.0, 0.0), 
            Point3D::new(0.5, 1.0, 0.5),
        ),
    ];
    
    let mesh = Mesh3D::new(triangles);
    let projector = ShadowProjector::new();
    
    // Test orthographic Z projection (top view)
    let shadow_paths = projector.orthographic_z(&mesh);
    assert!(!shadow_paths.is_empty());
    
    // Test front view projection
    let shadow_paths = projector.front_view(&mesh);
    assert!(!shadow_paths.is_empty());
}

#[test]
fn test_slice_to_toolpath_basic() {
    let triangles = vec![
        Triangle3D::new(
            Point3D::new(0.0, 0.0, 0.0),
            Point3D::new(2.0, 0.0, 0.0),
            Point3D::new(1.0, 2.0, 0.0),
        ),
    ];
    
    let mesh = Mesh3D::new(triangles);
    let mut converter = SliceToToolpath::new();
    
    // Generate shadow and convert to toolpath  
    let projector = ShadowProjector::new();
    let shadow_paths = projector.orthographic_z(&mesh);
    
    if let Some(path) = shadow_paths.first() {
        let toolpaths = converter.convert_slice_to_toolpath(
            path, 
            SliceStrategy::Contour,
            1.0, // layer_height  
        ).expect("Should generate toolpath");
        
        assert!(!toolpaths.is_empty());
    }
}

#[test]
fn test_stl_mesh_conversion() {
    // Create a simple triangle mesh
    let vertices = vec![
        [0.0, 0.0, 0.0],
        [1.0, 0.0, 0.0], 
        [0.5, 1.0, 0.0],
    ];
    
    let faces = vec![[0, 1, 2]];
    let indexed_mesh = stl_io::IndexedMesh { vertices, faces };
    
    // Convert to our Mesh3D format
    let mesh3d = Mesh3D::from_stl_mesh(&indexed_mesh);
    assert_eq!(mesh3d.triangle_count(), 1);
    
    // Test shadow projection from STL
    let projector = ShadowProjector::new();
    let shadow_paths = projector.orthographic_z(&mesh3d);
    assert!(!shadow_paths.is_empty());
}