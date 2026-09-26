struct Uniforms {
    offset: vec2f,
};

struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) uv: vec2f,
};

@group(0) @binding(0) var<uniform> uni: Uniforms;

const trianglePos = array<vec2f, 3>(
    vec2f(0.0, 0.6),
    vec2f(-0.5, -0.4),
    vec2f(0.5, -0.4),
);

@vertex
fn vs_main(@builtin(vertex_index) vertexIndex: u32) -> VertexOutput {
    var out: VertexOutput;
    let p = trianglePos[vertexIndex];
    out.position = vec4f(p + uni.offset, 0.0, 1.0);
    // uv 是“粘在三角形上”的坐标，插值后传给片段着色器
    out.uv = vec2f(p.x + 0.5, p.y + 0.4);
    return out;
}

// 有问题的版本：用画布像素坐标生成棋盘格
@fragment
fn fs_canvas(@builtin(position) pos: vec4f) -> @location(0) vec4f {
    let grid = vec2u(pos.xy / 24.0);   // 每 24 个物理像素一格
    let check = (grid.x + grid.y) % 2u;
    return select(vec4f(0.2, 0.2, 0.8, 1.0), vec4f(0.9), check == 1u);
}

// 修正版：用三角形自身的 uv 生成棋盘格
@fragment
fn fs_uv(in: VertexOutput) -> @location(0) vec4f {
    let grid = vec2u(in.uv * 8.0);     // 三角形内 8x8 格
    let check = (grid.x + grid.y) % 2u;
    return select(vec4f(0.8, 0.2, 0.2, 1.0), vec4f(0.9, 0.9, 0.2, 1.0), check == 1u);
}
