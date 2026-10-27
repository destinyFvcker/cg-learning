struct CameraUniform {
    view_proj: mat4x4<f32>,
}

// 因为在这里创建了两个BindGroup，所以需要进行区分，对应的顺序在render_pipeline_layout
// 之中指定，因为camera_bind_layout在索引1的位置，所以这里使用@group(1)
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.view_proj * vec4<f32>(model.position, 1.0); // 2.
    return out;
}

// 纹理对应的BindGroup在pipeline_layout的索引0位置，所以这里使用@group(0)
// 纹理和采样器通常配套使用：纹理提供图像数据，采样器决定如何读取这些数据
@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

// 进入片元着色器时，in.tex_coords 已经是GPU对三角形内部插值后的结果，
// 用它去采样纹理，就能拿到当前片元在贴图上对应位置的颜色
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(t_diffuse, s_diffuse, in.tex_coords);
}
