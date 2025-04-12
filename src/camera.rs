use glam::{Mat4, Vec3};

pub struct OrthographicCamera {
    pub projection_matrix: Mat4,
    // You might want to store additional information like viewport size, etc.
}

impl OrthographicCamera {
    pub fn new(left: f32, right: f32, bottom: f32, top: f32, near: f32, far: f32) -> Self {
        let projection_matrix = Mat4::orthographic_rh(left, right, bottom, top, near, far);
        OrthographicCamera { projection_matrix }
    }
    // ... (update_projection method will be explained later)
    pub fn update_projection(
        &mut self,
        left: f32,
        right: f32,
        bottom: f32,
        top: f32,
        near: f32,
        far: f32,
    ) {
        self.projection_matrix = Mat4::orthographic_rh(left, right, bottom, top, near, far);
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    projection: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            projection: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
    pub fn update_view_proj(&mut self, camera: &OrthographicCamera) {
        self.projection = camera.projection_matrix.to_cols_array_2d();
    }
}
