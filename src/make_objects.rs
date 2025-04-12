#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    z_index: f32,
}

const INDICES: &[u16] = &[
    0, 2, 1, // First triangle
    1, 2, 3, // Second triangle
];
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.5, 0.5],
        tex_coords: [0.0, 0.0],
        z_index: 0.0,
    },
    // Top-right
    Vertex {
        position: [0.5, 0.5],
        tex_coords: [1.0, 0.0],
        z_index: 0.0,
    },
    // Bottom-left
    Vertex {
        position: [-0.5, -0.5],
        tex_coords: [0.0, 1.0],
        z_index: 0.0,
    },
    // Bottom-right
    Vertex {
        position: [0.5, -0.5],
        tex_coords: [1.0, 1.0],
        z_index: 0.0,
    },
];
