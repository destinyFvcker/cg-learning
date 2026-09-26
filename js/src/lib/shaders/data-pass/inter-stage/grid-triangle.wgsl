struct OurVertexShaderOutput {
    @builtin(position) position: vec4f,
}

@vertex
fn vs_main(@builtin(vertex_index) VertexIndex: u32) -> OurVertexShaderOutput {
    let pos = array(
        vec2f(0.0, 0.5),
        vec2f(-0.5, -0.5),
        vec2f(0.5, -0.5),
    );

    var vsOutput: OurVertexShaderOutput;
    vsOutput.position = vec4f(pos[VertexIndex], 0.0, 1.0);
    return vsOutput;
}

@fragment
fn fs_main(fsInput: OurVertexShaderOutput) -> @location(0) vec4f {
    let red = vec4f(1, 0, 0, 1);
    let cyan = vec4f(0, 1, 1, 1);

    // vec2u实际上是vec2<u32>的简写，这里将浮点数阶段转换成整数像素坐标
    // .xy实际上等价于手动构造一个新向量
    // /8向下取整的效果是把像素坐标按照8x8的快分组，同组内所有像素得到
    // 相同的grid值
    let grid = vec2u(fsInput.position.xy) / 8;
    let checker = (grid.x + grid.y) % 2 == 1;

    return select(red, cyan, checker);
}
