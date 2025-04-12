// Instances


struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
    @location(9) tex_coord_bounds: vec4<f32>,
    @location(10) texture_index: u32,
};


struct VertexInput {
    @location(0) position: vec3<f32>, // Changed to vec2
    @location(1) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) @interpolate(flat) texture_index: u32,
};


// Vertex shader

@group(1) @binding(0) // 1.
var<uniform> camera: CameraUniform;
struct CameraUniform {
    projection: mat4x4<f32>,
};


@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
  let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.tex_coords = instance.tex_coord_bounds.xy + model.tex_coords * (instance.tex_coord_bounds.zw - instance.tex_coord_bounds.xy);
    out.clip_position = camera.projection *  model_matrix * vec4<f32>(model.position.xy, model.position.z, 1.0); 
    out.texture_index = instance.texture_index;
    return out;
}

// Fragment shader

@group(0) @binding(0)
var t_diffuse: texture_2d_array<f32>;

@group(0) @binding(1)
var s_diffuse: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords, in.texture_index);
}
