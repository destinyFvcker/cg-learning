/*
首先，我们声明一个struct来存储顶点着色器的输出。这目前就只有一个字段，也就是顶点的clip_position 。 
@builtin(position) 部分告诉 WGPU，这是我们想要用作顶点裁剪坐标的值。这类似于 GLSL 中的 gl_Position 变量。

gl_Position 是 GLSL 顶点着色器的内置输出变量，用于表示当前顶点的裁剪空间坐标（Clip Space）。
*/
struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    /*
    使用 var 定义的变量可以修改，但必须指定其类型。使用 let 创建的变量可以自动推断类型，但其值在着色器运行期间不可更改。
     */
    var out: VertexOutput;
    let x = f32(1 - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);

    /* 
    这里写入的 clip_position 是裁剪空间坐标。GPU 随后会进行透视除法： 
    NDC = clip_position.xyz / clip_position.w
    由于代码中 w = 1.0，所以 NDC 坐标就是 (x, y, 0)。随后 GPU 会把 NDC 映射到视口/帧缓冲像素坐标。
    */
    return out;
}

/* 
Fragment shader

@location(0)这个标记告诉WGPU将此函数返回的vec4值存储到第一个颜色目标之中。我们稍后讨论这一点。

关于 @buildtin(position) 的一点需要注意：在片段着色器之中，该值处于帧缓冲空间。这意味着如果你的窗口是
800 x 600，那么这里x和y的值将会分别介于0-800和0-600之间。y=0对应屏幕顶部
*/ 
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(0.3, 0.2, 0.1, 1.0);
}
