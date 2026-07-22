//! 缓冲区（Buffer）是GPU上的一块数据。缓冲区保证是连续的，这意味着所有的数据都将会按照顺序存储在内存之中。
//! 缓冲区通常用于存储结构体或数组等简单内容，但是也可以存储更加复杂的内容，例如树等图结构（前提是所有节点都存储在一起，
//! 且不引用缓冲区之外的任何内容）。
//!
//! 我们将大量使用缓冲区，所以让我们从最重要的两点开始：顶点缓冲区（Vertex Buffer）和索引缓冲区（Index Buffer）

// 之前我们直接在顶点着色器之中存储顶点类数据。这虽然在起步阶段运行良好，但是从长远上来看并不可行。
// 我们需要绘制的对象类型在规模上各不相同，而且，每当需要更新模型时都要重新编译着色器会极大降低程序的运行速度。

use std::sync::Arc;

use winit::window::Window;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct Vertex {
    /// 位置代表顶点在三维空间中的x、y和z坐标
    position: [f32; 3],
    /// 颜色就是顶点的红绿蓝数值
    color: [f32; 3],
}

/// 构成三角形的实际数据
///
/// 我们按照逆时针顺序排列顶点：顶部、左下、右下，这样做部分是出于传统，
/// 但是主要是因为在render_pipeline之中的primitive之中指定了希望三角形
/// 的front_face是wgpu::FrontFace::Ccw（Counter-Clockwise，逆时针），
/// 以便剔除背面。
///
/// 这意味着任何面向我们的三角形都应该使其顶点按照逆时针顺序排列
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.5, 0.0],
        color: [1.0, 0.0, 0.0],
    },
    Vertex {
        position: [-0.5, -0.5, 0.0],
        color: [0.0, 1.0, 0.0],
    },
    Vertex {
        position: [0.5, -0.5, 0.0],
        color: [0.0, 0.0, 1.0],
    },
];

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    is_surface_configured: bool,
    window: Arc<Window>,
}
