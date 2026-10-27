struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

/* 
Fragment shader

为什么三角形中间会产生渐变
三个顶点分别是：
顶部：红色 (1, 0, 0)
左下：绿色 (0, 1, 0)
右下：蓝色 (0, 0, 1)

顶点着色器只处理三个顶点。三角形内部各个片元的颜色由 GPU 自动插值：
红色顶点
   │
   │ 红色逐渐减少
   │ 绿色、蓝色逐渐增加
   ▼
三角形内部的混合色

片元着色器收到的 in.color 通常已经不是某个顶点的原始颜色，而是插值后的颜色：
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

最后加上 Alpha 1.0，写入第 0 个颜色附件，也就是 Rust 端 targets[0] 对应的 Surface 纹理。
*/ 

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}

// 线框叠加层使用固定的深色，避免和粉色填充混在一起。
@fragment
fn fs_line_main(_in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.03, 0.03, 0.03, 1.0);
}
