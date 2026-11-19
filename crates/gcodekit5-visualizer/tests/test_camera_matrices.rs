//! Pins the 3D camera's view and projection matrices to the formulas of the
//! glam 0.29 `Mat4::look_at_rh`, `Mat4::perspective_rh` and
//! `Mat4::orthographic_rh` methods, which glam 0.34 replaced with the
//! `glam::camera` module. The camera must render exactly as before.

use gcodekit5_visualizer::visualizer::Camera3D;
use glam::{Mat4, Vec3, Vec4};

/// glam 0.29 `Mat4::look_to_rh(eye, center - eye, up)`.
fn old_look_at_rh(eye: Vec3, center: Vec3, up: Vec3) -> Mat4 {
    let f = (center - eye).normalize();
    let s = f.cross(up).normalize();
    let u = s.cross(f);
    Mat4::from_cols(
        Vec4::new(s.x, u.x, -f.x, 0.0),
        Vec4::new(s.y, u.y, -f.y, 0.0),
        Vec4::new(s.z, u.z, -f.z, 0.0),
        Vec4::new(-eye.dot(s), -eye.dot(u), eye.dot(f), 1.0),
    )
}

/// glam 0.29 `Mat4::perspective_rh` (depth range `[0, 1]`).
fn old_perspective_rh(fov_y: f32, aspect: f32, near: f32, far: f32) -> Mat4 {
    let (sin_fov, cos_fov) = (0.5 * fov_y).sin_cos();
    let h = cos_fov / sin_fov;
    let w = h / aspect;
    let r = far / (near - far);
    Mat4::from_cols(
        Vec4::new(w, 0.0, 0.0, 0.0),
        Vec4::new(0.0, h, 0.0, 0.0),
        Vec4::new(0.0, 0.0, r, -1.0),
        Vec4::new(0.0, 0.0, r * near, 0.0),
    )
}

/// glam 0.29 `Mat4::orthographic_rh` (depth range `[0, 1]`).
fn old_orthographic_rh(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Mat4 {
    let rcp_width = 1.0 / (right - left);
    let rcp_height = 1.0 / (top - bottom);
    let r = 1.0 / (near - far);
    Mat4::from_cols(
        Vec4::new(rcp_width + rcp_width, 0.0, 0.0, 0.0),
        Vec4::new(0.0, rcp_height + rcp_height, 0.0, 0.0),
        Vec4::new(0.0, 0.0, r, 0.0),
        Vec4::new(
            -(left + right) * rcp_width,
            -(top + bottom) * rcp_height,
            r * near,
            1.0,
        ),
    )
}

fn camera(yaw_deg: f32, pitch_deg: f32) -> Camera3D {
    let mut camera = Camera3D::new(Vec3::new(12.0, -7.5, 3.25), 180.0);
    camera.aspect_ratio = 1.6;
    camera.set_view(yaw_deg, pitch_deg);
    camera
}

fn assert_matrix_eq(actual: Mat4, expected: Mat4) {
    assert!(
        actual.abs_diff_eq(expected, 1e-4),
        "matrix differs:\n actual   {actual:?}\n expected {expected:?}"
    );
}

#[test]
fn test_camera_view_matrix_matches_glam_0_29() {
    // An oblique view uses Z as up.
    let oblique = camera(30.0, 20.0);
    assert_matrix_eq(
        oblique.get_view_matrix(),
        old_look_at_rh(oblique.get_eye_position(), oblique.target, Vec3::Z),
    );

    // Looking straight down switches to Y as up.
    let top = camera(-45.0, 90.0);
    assert_matrix_eq(
        top.get_view_matrix(),
        old_look_at_rh(top.get_eye_position(), top.target, Vec3::Y),
    );
}

#[test]
fn test_camera_perspective_projection_matches_glam_0_29() {
    // 20 degrees of pitch is not one of the snapped orthographic views.
    let camera = camera(30.0, 20.0);
    assert_matrix_eq(
        camera.get_projection_matrix(),
        old_perspective_rh(
            camera.fov.to_radians(),
            camera.aspect_ratio,
            camera.near,
            camera.far,
        ),
    );
}

#[test]
fn test_camera_orthographic_projection_matches_glam_0_29() {
    let mut camera = camera(0.0, 0.0);
    camera.set_isometric();
    let half_height = camera.distance * (camera.fov.to_radians() / 2.0).tan();
    let half_width = half_height * camera.aspect_ratio;
    assert_matrix_eq(
        camera.get_projection_matrix(),
        old_orthographic_rh(
            -half_width,
            half_width,
            -half_height,
            half_height,
            camera.near,
            camera.far,
        ),
    );
}
