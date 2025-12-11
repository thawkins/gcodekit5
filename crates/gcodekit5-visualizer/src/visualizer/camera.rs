use glam::{Mat4, Vec3};

#[derive(Debug, Clone, Copy)]
pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,   // radians
    pub pitch: f32, // radians
    pub fov: f32,   // degrees
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
    pub min_distance: f32,
    pub max_distance: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::ZERO,
            distance: 100.0,
            yaw: -45.0f32.to_radians(),
            pitch: 45.0f32.to_radians(),
            fov: 45.0,
            aspect_ratio: 1.0,
            near: 0.1,
            far: 1000.0,
            min_distance: 1.0,
            max_distance: 1000.0,
        }
    }
}

impl Camera {
    pub fn new(target: Vec3, distance: f32) -> Self {
        Self {
            target,
            distance,
            ..Default::default()
        }
    }

    pub fn update_aspect_ratio(&mut self, width: f32, height: f32) {
        if height > 0.0 {
            self.aspect_ratio = width / height;
        }
    }

    pub fn orbit(&mut self, delta_yaw: f32, delta_pitch: f32) {
        self.yaw += delta_yaw;
        self.pitch += delta_pitch;

        // Clamp pitch to avoid gimbal lock or flipping
        // Keep it between -89 and 89 degrees roughly
        let limit = 89.0f32.to_radians();
        self.pitch = self.pitch.clamp(-limit, limit);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance -= delta;
        self.distance = self.distance.clamp(self.min_distance, self.max_distance);
    }

    pub fn pan(&mut self, delta_x: f32, delta_y: f32) {
        // Calculate right and up vectors relative to camera view
        // Re-derive basis vectors from yaw/pitch
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        // Forward vector (camera to target)
        // Using Z-up convention:
        // x = r * cos(pitch) * cos(yaw)
        // y = r * cos(pitch) * sin(yaw)
        // z = r * sin(pitch)

        let offset_dir = Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch).normalize();

        // Camera Forward is -offset_dir (looking at target)
        let forward = -offset_dir;

        // Handle singularity when looking straight up/down
        let world_up = if forward.cross(Vec3::Z).length_squared() < 0.001 {
            Vec3::Y
        } else {
            Vec3::Z
        };

        // Camera Right
        let cam_right = forward.cross(world_up).normalize();

        // Camera Up
        let cam_up = cam_right.cross(forward).normalize();

        // Scale factor for panning speed
        let scale = self.distance * 0.001;

        // Move target
        self.target -= cam_right * delta_x * scale;
        self.target += cam_up * delta_y * scale;
    }

    pub fn get_eye_position(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();

        // Z-up convention
        let offset = Vec3::new(cos_pitch * cos_yaw, cos_pitch * sin_yaw, sin_pitch) * self.distance;

        self.target + offset
    }

    pub fn get_view_matrix(&self) -> Mat4 {
        let eye = self.get_eye_position();
        let forward = (self.target - eye).normalize();

        // Check if forward is parallel to Z (up)
        let up = if forward.cross(Vec3::Z).length_squared() < 0.001 {
            // Forward is vertical, use Y as up
            // If looking down (-Z), Y is up on screen (Top of screen points to +Y world)
            // If looking up (+Z), Y is down?
            // Let's stick to Y as up for vertical views
            Vec3::Y
        } else {
            Vec3::Z
        };

        Mat4::look_at_rh(eye, self.target, up)
    }

    pub fn set_view(&mut self, yaw_deg: f32, pitch_deg: f32) {
        self.yaw = yaw_deg.to_radians();
        self.pitch = pitch_deg.to_radians();
    }

    pub fn set_isometric(&mut self) {
        self.yaw = -45.0f32.to_radians();
        self.pitch = 35.264f32.to_radians(); // Standard isometric pitch
    }

    pub fn get_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(
            self.fov.to_radians(),
            self.aspect_ratio,
            self.near,
            self.far,
        )
    }

    pub fn fit_to_bounds(&mut self, min: Vec3, max: Vec3) {
        let center = (min + max) * 0.5;
        let size = max - min;
        let max_dim = size.max_element();

        self.target = center;

        // Calculate distance needed to see the object
        // tan(fov/2) = (size/2) / distance
        // distance = (size/2) / tan(fov/2)
        let fov_rad = self.fov.to_radians();
        let distance = (max_dim * 1.2) / (fov_rad / 2.0).tan(); // 1.2 factor for margin

        self.distance = distance.clamp(self.min_distance, self.max_distance);
    }
}
