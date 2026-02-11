//! # Parametric Shape Generators
//!
//! Provides generators for complex mechanical and structural shapes.

use crate::model::Point;
use lyon::math::point;
use lyon::path::Path;
use std::f64::consts::PI;

/// Generate a spur gear path
pub fn generate_spur_gear(
    center: Point,
    module: f64,
    teeth: usize,
    pressure_angle_deg: f64,
    hole_radius: f64,
) -> Path {
    let mut builder = Path::builder();

    let pitch_radius = (module * teeth as f64) / 2.0;
    let addendum = module;
    let dedendum = 1.25 * module;
    let outer_radius = pitch_radius + addendum;
    let root_radius = pitch_radius - dedendum;
    let base_radius = pitch_radius * pressure_angle_deg.to_radians().cos();

    let angle_per_tooth = 2.0 * PI / teeth as f64;
    let _half_tooth_angle = angle_per_tooth / 4.0;

    // Involute curve calculation
    // x = r_b * (cos(t) + t * sin(t))
    // y = r_b * (sin(t) - t * cos(t))

    let mut points = Vec::new();

    for i in 0..teeth {
        let tooth_center_angle = i as f64 * angle_per_tooth;

        // We need to find the angle 't' where the involute reaches the outer radius
        // r^2 = r_b^2 * (1 + t^2) => t = sqrt((r/r_b)^2 - 1)
        let t_max = ((outer_radius / base_radius).powi(2) - 1.0).sqrt();
        let steps = 5;

        // Left side of tooth (involute)
        let mut tooth_points = Vec::new();
        for j in 0..=steps {
            let t = (j as f64 / steps as f64) * t_max;
            let x = base_radius * (t.cos() + t * t.sin());
            let y = base_radius * (t.sin() - t * t.cos());

            // The involute starts at the base circle. If root circle is smaller, we add a radial line.
            let r = (x * x + y * y).sqrt();
            let phi = y.atan2(x);

            // Rotate to tooth position
            // We need to offset the involute so the tooth has the correct thickness at the pitch circle.
            // Thickness at pitch circle is pitch_radius * half_tooth_angle * 2? No, it's circular pitch / 2.
            // Circular pitch = PI * module. Tooth thickness = PI * module / 2.
            // Angle for tooth thickness = (PI * module / 2) / pitch_radius = PI / teeth.
            // So half thickness angle = PI / (2 * teeth).

            // Angle of involute at pitch radius
            let t_pitch = ((pitch_radius / base_radius).powi(2) - 1.0).sqrt();
            let phi_pitch = (t_pitch.sin() - t_pitch * t_pitch.cos())
                .atan2(t_pitch.cos() + t_pitch * t_pitch.sin());

            let angle = tooth_center_angle - (PI / (2.0 * teeth as f64)) - phi_pitch + phi;

            tooth_points.push(Point::new(
                center.x + r * angle.cos(),
                center.y + r * angle.sin(),
            ));
        }

        // If root radius is smaller than base radius, add a point at root
        if root_radius < base_radius {
            let p0 = tooth_points[0];
            let _r0 = (p0.x - center.x).hypot(p0.y - center.y);
            let angle0 = (p0.y - center.y).atan2(p0.x - center.x);
            points.push(Point::new(
                center.x + root_radius * angle0.cos(),
                center.y + root_radius * angle0.sin(),
            ));
        }

        points.extend(tooth_points.clone());

        // Top of tooth (arc)
        // Right side of tooth (mirrored involute)
        let mut right_points = Vec::new();
        for j in (0..=steps).rev() {
            let t = (j as f64 / steps as f64) * t_max;
            let x = base_radius * (t.cos() + t * t.sin());
            let y = -(base_radius * (t.sin() - t * t.cos())); // Mirror Y

            let r = (x * x + y * y).sqrt();
            let phi = y.atan2(x);

            let t_pitch = ((pitch_radius / base_radius).powi(2) - 1.0).sqrt();
            let phi_pitch = (t_pitch.sin() - t_pitch * t_pitch.cos())
                .atan2(t_pitch.cos() + t_pitch * t_pitch.sin());

            let angle = tooth_center_angle + (PI / (2.0 * teeth as f64)) + phi_pitch + phi;

            right_points.push(Point::new(
                center.x + r * angle.cos(),
                center.y + r * angle.sin(),
            ));
        }
        points.extend(right_points);

        if root_radius < base_radius {
            if let Some(p_last) = points.last() {
                let angle_last = (p_last.y - center.y).atan2(p_last.x - center.x);
                points.push(Point::new(
                    center.x + root_radius * angle_last.cos(),
                    center.y + root_radius * angle_last.sin(),
                ));
            }
        }
    }

    // Build the path
    if !points.is_empty() {
        builder.begin(point(points[0].x as f32, points[0].y as f32));
        for p in points.iter().skip(1) {
            builder.line_to(point(p.x as f32, p.y as f32));
        }
        builder.close();
    }

    // Add center hole if requested
    if hole_radius > 0.0 {
        builder.add_circle(
            point(center.x as f32, center.y as f32),
            hole_radius as f32,
            lyon::path::Winding::Negative,
        );
    }

    builder.build()
}

/// Generate a sprocket path (ANSI standard-ish)
pub fn generate_sprocket(
    center: Point,
    pitch: f64,
    teeth: usize,
    roller_diameter: f64,
    hole_radius: f64,
) -> Path {
    let mut builder = Path::builder();

    let pitch_radius = pitch / (2.0 * (PI / teeth as f64).sin());
    let roller_radius = roller_diameter / 2.0;

    // Simplified sprocket geometry: arcs for rollers and straight lines or arcs for teeth
    let mut points = Vec::new();
    let angle_per_tooth = 2.0 * PI / teeth as f64;

    for i in 0..teeth {
        let angle = i as f64 * angle_per_tooth;

        // Roller center
        let rc_x = center.x + pitch_radius * angle.cos();
        let rc_y = center.y + pitch_radius * angle.sin();

        // We'll approximate the sprocket with a series of points
        // In a real sprocket, the tooth shape is more complex (seated curve)
        // For now, let's do a simple version:
        // 1. Arc around the roller
        // 2. Tip of the tooth

        let steps = 4;
        for j in 0..=steps {
            let a = angle + PI + (j as f64 / steps as f64 - 0.5) * (PI * 0.8); // Arc around roller
            points.push(Point::new(
                rc_x + roller_radius * a.cos(),
                rc_y + roller_radius * a.sin(),
            ));
        }

        // Tip of tooth (between rollers)
        let tip_angle = angle + angle_per_tooth / 2.0;
        let tip_radius = pitch_radius + roller_radius * 0.8;
        points.push(Point::new(
            center.x + tip_radius * tip_angle.cos(),
            center.y + tip_radius * tip_angle.sin(),
        ));
    }

    if !points.is_empty() {
        builder.begin(point(points[0].x as f32, points[0].y as f32));
        for p in points.iter().skip(1) {
            builder.line_to(point(p.x as f32, p.y as f32));
        }
        builder.close();
    }

    if hole_radius > 0.0 {
        builder.add_circle(
            point(center.x as f32, center.y as f32),
            hole_radius as f32,
            lyon::path::Winding::Negative,
        );
    }

    builder.build()
}

/// Generate a tabbed box (finger joints)
/// Returns a vector of paths, one for each face (unfolded)
pub fn generate_tabbed_box(
    width: f64,
    height: f64,
    depth: f64,
    thickness: f64,
    tab_width: f64,
) -> Vec<Path> {
    let mut paths = Vec::new();

    // Helper to generate a face with tabs
    // edges: [top, right, bottom, left] - true if tabs are "out", false if "in"
    let generate_face = |w: f64, h: f64, edges: [bool; 4]| -> Path {
        let mut builder = Path::builder();

        let mut current_x = 0.0;
        let mut current_y = 0.0;
        builder.begin(point(current_x as f32, current_y as f32));

        // Top edge (0 to w, y=0)
        let num_tabs_w = (w / tab_width).floor() as usize;
        if num_tabs_w.is_multiple_of(2) { /* ensure odd for symmetry? */ }
        let actual_tab_w = w / num_tabs_w as f64;

        for i in 0..num_tabs_w {
            let is_tab = i % 2 == 0;
            let out = if edges[0] { is_tab } else { !is_tab };

            if out {
                builder.line_to(point(current_x as f32, (current_y - thickness) as f32));
                current_x += actual_tab_w;
                builder.line_to(point(current_x as f32, (current_y - thickness) as f32));
                builder.line_to(point(current_x as f32, current_y as f32));
            } else {
                current_x += actual_tab_w;
                builder.line_to(point(current_x as f32, current_y as f32));
            }
        }

        // Right edge (x=w, 0 to h)
        let num_tabs_h = (h / tab_width).floor() as usize;
        let actual_tab_h = h / num_tabs_h as f64;
        for i in 0..num_tabs_h {
            let is_tab = i % 2 == 0;
            let out = if edges[1] { is_tab } else { !is_tab };

            if out {
                builder.line_to(point((current_x + thickness) as f32, current_y as f32));
                current_y += actual_tab_h;
                builder.line_to(point((current_x + thickness) as f32, current_y as f32));
                builder.line_to(point(current_x as f32, current_y as f32));
            } else {
                current_y += actual_tab_h;
                builder.line_to(point(current_x as f32, current_y as f32));
            }
        }

        // Bottom edge (w to 0, y=h)
        for i in 0..num_tabs_w {
            let is_tab = i % 2 == 0;
            let out = if edges[2] { is_tab } else { !is_tab };

            if out {
                builder.line_to(point(current_x as f32, (current_y + thickness) as f32));
                current_x -= actual_tab_w;
                builder.line_to(point(current_x as f32, (current_y + thickness) as f32));
                builder.line_to(point(current_x as f32, current_y as f32));
            } else {
                current_x -= actual_tab_w;
                builder.line_to(point(current_x as f32, current_y as f32));
            }
        }

        // Left edge (x=0, h to 0)
        for i in 0..num_tabs_h {
            let is_tab = i % 2 == 0;
            let out = if edges[3] { is_tab } else { !is_tab };

            if out {
                builder.line_to(point((current_x - thickness) as f32, current_y as f32));
                current_y -= actual_tab_h;
                builder.line_to(point((current_x - thickness) as f32, current_y as f32));
                builder.line_to(point(current_x as f32, current_y as f32));
            } else {
                current_y -= actual_tab_h;
                builder.line_to(point(current_x as f32, current_y as f32));
            }
        }

        builder.close();
        builder.build()
    };

    // Layout the 6 faces
    // Bottom
    let bottom = generate_face(width, depth, [true, true, true, true]);
    paths.push(bottom);

    // Top (offset by depth + padding)
    let top = generate_face(width, depth, [true, true, true, true]);
    let t_top = lyon::math::Transform::translation(0.0, (depth + 10.0) as f32);
    paths.push(top.transformed(&t_top));

    // Front
    let front = generate_face(width, height, [false, true, false, true]);
    let t_front = lyon::math::Transform::translation(0.0, (2.0 * depth + 20.0) as f32);
    paths.push(front.transformed(&t_front));

    // Back
    let back = generate_face(width, height, [false, true, false, true]);
    let t_back = lyon::math::Transform::translation(0.0, (2.0 * depth + height + 30.0) as f32);
    paths.push(back.transformed(&t_back));

    // Left
    let left = generate_face(depth, height, [false, false, false, false]);
    let t_left =
        lyon::math::Transform::translation((width + 10.0) as f32, (2.0 * depth + 20.0) as f32);
    paths.push(left.transformed(&t_left));

    // Right
    let right = generate_face(depth, height, [false, false, false, false]);
    let t_right = lyon::math::Transform::translation(
        (width + depth + 20.0) as f32,
        (2.0 * depth + 20.0) as f32,
    );
    paths.push(right.transformed(&t_right));

    paths
}
pub fn generate_helical_gear(
    center: Point,
    module: f64,
    teeth: usize,
    pressure_angle_deg: f64,
    helix_angle_deg: f64,
    hole_radius: f64,
) -> Path {
    // For simplicity, we'll generate a basic involute gear and add a helical offset
    // A true helical gear would have the involute curve skewed along the helix
    // This is a simplified approximation

    let mut builder = Path::builder();

    let pitch_radius = (module * teeth as f64) / 2.0;
    let addendum = module;
    let dedendum = 1.25 * module;
    let outer_radius = pitch_radius + addendum;
    let root_radius = pitch_radius - dedendum;
    let base_radius = pitch_radius * pressure_angle_deg.to_radians().cos();

    let angle_per_tooth = 2.0 * PI / teeth as f64;
    let helix_offset = helix_angle_deg.to_radians().tan() * module; // Simplified helix offset

    let mut points = Vec::new();

    for i in 0..teeth {
        let tooth_center_angle = i as f64 * angle_per_tooth;

        let t_max = ((outer_radius / base_radius).powi(2) - 1.0).sqrt();
        let steps = 5;

        let mut tooth_points = Vec::new();
        for j in 0..=steps {
            let t = (j as f64 / steps as f64) * t_max;
            let x = base_radius * (t.cos() + t * t.sin());
            let y = base_radius * (t.sin() - t * t.cos());

            let r = (x * x + y * y).sqrt();
            let phi = y.atan2(x);

            let t_pitch = ((pitch_radius / base_radius).powi(2) - 1.0).sqrt();
            let phi_pitch = (t_pitch.sin() - t_pitch * t_pitch.cos())
                .atan2(t_pitch.cos() + t_pitch * t_pitch.sin());

            let angle = tooth_center_angle - (PI / (2.0 * teeth as f64)) - phi_pitch + phi;

            // Add helical offset
            let helical_angle =
                angle + (r - root_radius) * helix_offset / (outer_radius - root_radius);

            tooth_points.push(Point::new(
                center.x + r * helical_angle.cos(),
                center.y + r * helical_angle.sin(),
            ));
        }

        if root_radius < base_radius {
            let p0 = tooth_points[0];
            let angle0 = (p0.y - center.y).atan2(p0.x - center.x);
            points.push(Point::new(
                center.x + root_radius * angle0.cos(),
                center.y + root_radius * angle0.sin(),
            ));
        }

        points.extend(tooth_points.clone());

        let mut right_points = Vec::new();
        for j in (0..=steps).rev() {
            let t = (j as f64 / steps as f64) * t_max;
            let x = base_radius * (t.cos() + t * t.sin());
            let y = -(base_radius * (t.sin() - t * t.cos()));

            let r = (x * x + y * y).sqrt();
            let phi = y.atan2(x);

            let t_pitch = ((pitch_radius / base_radius).powi(2) - 1.0).sqrt();
            let phi_pitch = (t_pitch.sin() - t_pitch * t_pitch.cos())
                .atan2(t_pitch.cos() + t_pitch * t_pitch.sin());

            let angle = tooth_center_angle + (PI / (2.0 * teeth as f64)) + phi_pitch + phi;
            let helical_angle =
                angle + (r - root_radius) * helix_offset / (outer_radius - root_radius);

            right_points.push(Point::new(
                center.x + r * helical_angle.cos(),
                center.y + r * helical_angle.sin(),
            ));
        }
        points.extend(right_points);

        if root_radius < base_radius {
            if let Some(p_last) = points.last() {
                let angle_last = (p_last.y - center.y).atan2(p_last.x - center.x);
                points.push(Point::new(
                    center.x + root_radius * angle_last.cos(),
                    center.y + root_radius * angle_last.sin(),
                ));
            }
        }
    }

    if !points.is_empty() {
        builder.begin(point(points[0].x as f32, points[0].y as f32));
        for p in points.iter().skip(1) {
            builder.line_to(point(p.x as f32, p.y as f32));
        }
        builder.close();
    }

    if hole_radius > 0.0 {
        builder.add_circle(
            point(center.x as f32, center.y as f32),
            hole_radius as f32,
            lyon::path::Winding::Negative,
        );
    }

    builder.build()
}

/// Generate a timing pulley (XL series approximation)
pub fn generate_timing_pulley(
    center: Point,
    pitch: f64,
    teeth: usize,
    _belt_width: f64,
    hole_radius: f64,
) -> Path {
    let mut builder = Path::builder();

    // XL timing belt parameters (simplified)
    let tooth_height = 1.27; // mm

    let outer_radius = (pitch * teeth as f64) / (2.0 * PI) * (PI / teeth as f64).sin().recip();
    let root_radius = outer_radius - tooth_height;

    let mut points = Vec::new();
    let angle_per_tooth = 2.0 * PI / teeth as f64;

    for i in 0..teeth {
        let tooth_center_angle = i as f64 * angle_per_tooth;

        // Tooth tip
        let tip_angle = tooth_center_angle;
        points.push(Point::new(
            center.x + outer_radius * tip_angle.cos(),
            center.y + outer_radius * tip_angle.sin(),
        ));

        // Left flank
        let left_angle = tooth_center_angle - angle_per_tooth * 0.3;
        points.push(Point::new(
            center.x + root_radius * left_angle.cos(),
            center.y + root_radius * left_angle.sin(),
        ));

        // Bottom of tooth
        let bottom_angle = tooth_center_angle - angle_per_tooth * 0.2;
        points.push(Point::new(
            center.x + root_radius * bottom_angle.cos(),
            center.y + root_radius * bottom_angle.sin(),
        ));

        // Right flank
        let right_angle = tooth_center_angle + angle_per_tooth * 0.3;
        points.push(Point::new(
            center.x + root_radius * right_angle.cos(),
            center.y + root_radius * right_angle.sin(),
        ));
    }

    if !points.is_empty() {
        builder.begin(point(points[0].x as f32, points[0].y as f32));
        for p in points.iter().skip(1) {
            builder.line_to(point(p.x as f32, p.y as f32));
        }
        builder.close();
    }

    if hole_radius > 0.0 {
        builder.add_circle(
            point(center.x as f32, center.y as f32),
            hole_radius as f32,
            lyon::path::Winding::Negative,
        );
    }

    builder.build()
}

/// Generate a slot (rectangular cutout)
pub fn generate_slot(center: Point, length: f64, width: f64, corner_radius: f64) -> Path {
    let mut builder = Path::builder();

    let half_length = length / 2.0;
    let half_width = width / 2.0;

    if corner_radius > 0.0 {
        // Rounded rectangle slot
        let cr = corner_radius.min(half_width).min(half_length);
        builder.add_rounded_rectangle(
            &lyon::math::Box2D::new(
                point(
                    (center.x - half_length) as f32,
                    (center.y - half_width) as f32,
                ),
                point(
                    (center.x + half_length) as f32,
                    (center.y + half_width) as f32,
                ),
            ),
            &lyon::path::builder::BorderRadii::new(cr as f32),
            lyon::path::Winding::Positive,
        );
    } else {
        // Rectangular slot
        builder.add_rectangle(
            &lyon::math::Box2D::new(
                point(
                    (center.x - half_length) as f32,
                    (center.y - half_width) as f32,
                ),
                point(
                    (center.x + half_length) as f32,
                    (center.y + half_width) as f32,
                ),
            ),
            lyon::path::Winding::Positive,
        );
    }

    builder.build()
}

/// Generate an L-bracket
pub fn generate_l_bracket(
    center: Point,
    width: f64,
    height: f64,
    thickness: f64,
    hole_diameter: f64,
    hole_spacing: f64,
) -> Path {
    let mut builder = Path::builder();

    let half_width = width / 2.0;
    let half_height = height / 2.0;

    // Create L-shape
    let points = [
        Point::new(center.x - half_width, center.y - half_height),
        Point::new(center.x + half_width, center.y - half_height),
        Point::new(center.x + half_width, center.y - half_height + thickness),
        Point::new(
            center.x - half_width + thickness,
            center.y - half_height + thickness,
        ),
        Point::new(center.x - half_width + thickness, center.y + half_height),
        Point::new(center.x - half_width, center.y + half_height),
    ];

    builder.begin(point(points[0].x as f32, points[0].y as f32));
    for p in points.iter().skip(1) {
        builder.line_to(point(p.x as f32, p.y as f32));
    }
    builder.close();

    // Add mounting holes
    let hole_radius = hole_diameter / 2.0;
    let hole_centers = vec![
        Point::new(
            center.x - half_width + thickness / 2.0,
            center.y - half_height + thickness / 2.0,
        ),
        Point::new(
            center.x + half_width - hole_spacing,
            center.y - half_height + thickness / 2.0,
        ),
        Point::new(
            center.x - half_width + thickness / 2.0,
            center.y + half_height - hole_spacing,
        ),
    ];

    for hc in hole_centers {
        builder.add_circle(
            point(hc.x as f32, hc.y as f32),
            hole_radius as f32,
            lyon::path::Winding::Negative,
        );
    }

    builder.build()
}

/// Generate a U-bracket/channel
pub fn generate_u_bracket(
    center: Point,
    length: f64,
    width: f64,
    thickness: f64,
    hole_diameter: f64,
    hole_spacing: f64,
) -> Path {
    let mut builder = Path::builder();

    let half_length = length / 2.0;
    let half_width = width / 2.0;

    // Create U-shape
    let points = [
        Point::new(center.x - half_length, center.y - half_width),
        Point::new(center.x + half_length, center.y - half_width),
        Point::new(center.x + half_length, center.y + half_width),
        Point::new(center.x + half_length - thickness, center.y + half_width),
        Point::new(
            center.x + half_length - thickness,
            center.y - half_width + thickness,
        ),
        Point::new(
            center.x - half_length + thickness,
            center.y - half_width + thickness,
        ),
        Point::new(center.x - half_length + thickness, center.y + half_width),
        Point::new(center.x - half_length, center.y + half_width),
    ];

    builder.begin(point(points[0].x as f32, points[0].y as f32));
    for p in points.iter().skip(1) {
        builder.line_to(point(p.x as f32, p.y as f32));
    }
    builder.close();

    // Add mounting holes
    let hole_radius = hole_diameter / 2.0;
    let hole_centers = vec![
        Point::new(
            center.x - half_length + thickness / 2.0,
            center.y - half_width + thickness / 2.0,
        ),
        Point::new(
            center.x + half_length - thickness / 2.0,
            center.y - half_width + thickness / 2.0,
        ),
        Point::new(
            center.x - half_length + thickness / 2.0,
            center.y + half_width - hole_spacing,
        ),
        Point::new(
            center.x + half_length - thickness / 2.0,
            center.y + half_width - hole_spacing,
        ),
    ];

    for hc in hole_centers {
        builder.add_circle(
            point(hc.x as f32, hc.y as f32),
            hole_radius as f32,
            lyon::path::Winding::Negative,
        );
    }

    builder.build()
}
