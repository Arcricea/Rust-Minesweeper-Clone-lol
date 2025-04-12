use glam::{Mat4, Vec2, Vec4};

pub struct Instance {}

impl Instance {
    pub fn to_raw(
        position: Vec2,
        rotation: f32,
        scale: Vec2,
        z_index: f32,
        tex_coords_bounds: Vec4,
        texture_index: u32,
    ) -> InstanceRaw {
        let model = Mat4::from_scale_rotation_translation(
            scale.extend(1.0),
            glam::Quat::from_rotation_z(rotation),
            position.extend(z_index),
        );
        InstanceRaw {
            model: model.to_cols_array_2d(),
            texture_index,
            tex_coords_bounds: tex_coords_bounds.into(),
            z_index,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
    tex_coords_bounds: [f32; 4],
    texture_index: u32,
    pub z_index: f32,
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // tex_coords_bounds
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
                // texture_index
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 20]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Uint32,
                },
            ],
        }
    }
}
