//! 从技术上来说，我们并不一定需要索引缓冲区，但是它依旧非常有用。当我们开始使用具有大量三角形的模型的时候，
//! 索引缓冲区就派上用场了

#![allow(unused)]

/*
                          A----
                        /     --------
                       /               ---------
                     /                         --------
                   /                                   ----B
                  /                                ------ //|
                /                          -------     //  |
               /                     -------           /    |
             /                ------                //      |
            /           -------                     /        |
          /      -------                          //         |
        / ------                               //           |
       E---                                    /             |
         \\                                  //              |
           \\                              //                |
             \\                           /                   |
               \\                       //                    |
                 \\                   //                      |
                   \\                /                     ---C
                     \\            //                ------
                       \\         /           -------
                         \\     //     -------
                           \\ // ------
                             D---
*/

use std::sync::Arc;

use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

/// 对应上面的ASCII图，这里一共需要9个顶点，但是我们实际上画的就只是一个5边形，这就意味着
/// 有4个顶点被重复使用了。
///
/// ## 这里的颜色为什么是 `[0.5, 0.0, 0.5]`
///
/// 顶点缓冲区中的浮点颜色会被着色器当作线性 RGB；因此这里的 `0.5` 表示约一半的物理光强，
/// 而不是取色器中 `128 / 255` 的 sRGB 编码。写入 sRGB Surface 时，GPU 会将线性的 `0.5`
/// 编码成约 `0.735`，量化为 8 位后约为 `188`，所以取色器读到的颜色接近
/// `#BC00BC`，即 `(188, 0, 188)`。
///
/// sRGB 映射并没有扩展 `[0, 1]` 的范围，也没有创造更多颜色值。对于 8 位通道来说，总数始终
/// 只有 256 个编码；非线性映射只是重新分配这些有限的刻度：在线性亮度轴上让暗部刻度更密、
/// 亮部刻度更疏。这能以较多编码描述人眼更敏感的暗部变化，以较少编码描述不易察觉的亮部变化，
/// 从而让有限的量化精度更符合人眼感知。
///
/// 反过来解释取色器数值时，`188 / 255 ≈ 0.737` 只完成了从 `[0, 255]` 到 `[0, 1]`
/// 的归一化，数值仍在 sRGB 空间；继续进行 sRGB 解码后才得到线性值 `≈ 0.503`，
/// 与这里的 `0.5` 基本一致，微小差异来自 8 位整数的量化和舍入。
const VERTICES_NOT_GOOD: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
];

/// 这才是好的做法，通过使用索引缓冲区就能做到
///
/// 我们将所有唯一的顶点存储在 VERTICES 中，
/// 并创建另一个缓冲区来存储指向 VERTICES 中元素的索引，从而创建三角形
const VERTICES: &[Vertex] = &[
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        color: [0.5, 0.0, 0.5],
    }, // E
];

const INDICES: &[u16] = &[0, 1, 4, 1, 2, 4, 2, 3, 4];

// TriangleList 的每组三个索引组成一个三角形；这里把这些三角形的边转换成
// LineList 所需的成对索引。除了多边形外轮廓，也会显示三角形之间的内部连线。
const LINE_INDICES: &[u16] = &[
    0, 1, 1, 2, 2, 3, 3, 4, 4, 0, // 外轮廓
    1, 4, 2, 4, // 三角形内部连线
];

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    // vertex_attr_array! 会根据前一个属性的格式自动计算下一个属性的字节偏移：
    //
    // @location(0) position: Float32x3 -> offset = 0，大小为 3 * 4 = 12 字节
    // @location(1) color:    Float32x3 -> offset = 12，大小为 3 * 4 = 12 字节
    //
    // 这两个 location 必须与 buffer-indices-shader.wgsl 中 VertexInput 的
    // @location(0) position 和 @location(1) color 一一对应。
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            // 一个完整 Vertex 占用的字节数：position 12 字节 + color 12 字节 = 24 字节。
            // GPU 读取完一个顶点后，会向后移动 array_stride 字节，再读取下一个顶点。
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            // 每处理一个新顶点就移动一次，而不是每处理一个新实例才移动一次。
            step_mode: wgpu::VertexStepMode::Vertex,
            // 告诉 GPU：这 24 字节应当如何拆成着色器需要的两个输入属性。
            attributes: &Self::ATTRIBUTES,
        }
    }
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    line_index_buffer: wgpu::Buffer,
    num_indices: u32,
    num_line_indices: u32,
    is_surface_configured: bool,
    window: Arc<Window>,
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // 创建wgpu入口，此时不绑定窗口
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: if cfg!(target_arch = "wasm32") {
                wgpu::Backends::GL
            } else {
                wgpu::Backends::PRIMARY
            },
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&wgpu::wgt::DeviceDescriptor {
                label: Some("my divice"),
                required_features: wgpu::Features::empty(),
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::defaults()
                },
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await?;

        let surface_cap = surface.get_capabilities(&adapter);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // 优先选择 sRGB Surface 格式。若选择成功，片元着色器仍应输出线性 RGB，
            // GPU 会在写入 Surface 时自动执行 linear -> sRGB 编码。因此顶点颜色中的
            // 线性 0.5 最终会以约 0.735（8 位约为 188）的 sRGB 数值保存/显示。
            //
            // 注意：下面保留了非 sRGB 格式的回退路径；若实际回退到这种格式，则不会发生
            // 上述自动 sRGB 编码，不能再按同一条转换链理解最终数值。
            format: surface_cap
                .formats
                .iter()
                .find(|f| f.is_srgb())
                .copied()
                .unwrap_or(surface_cap.formats[0]),
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width,
            height: size.height,
            present_mode: surface_cap.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_cap.alpha_modes[0],
            view_formats: vec![],
        };

        let shader = device.create_shader_module(wgpu::include_wgsl!(
            "../media/shaders/buffer-indices-shader.wgsl"
        ));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::desc())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        // 线框使用独立的 pipeline：填充 pipeline 仍然绘制三角形，
        // 线框 pipeline 则把每两个索引解释成一条线。
        let line_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Line Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(Vertex::desc())],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_line_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let line_index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Index Buffer"),
            contents: bytemuck::cast_slice(LINE_INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;
        let num_line_indices = LINE_INDICES.len() as u32;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            line_pipeline,
            vertex_buffer,
            index_buffer,
            line_index_buffer,
            num_indices,
            num_line_indices,
            is_surface_configured: false,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true
        }
    }

    // 处理键盘实践，这里只是在按下退出键的时候退出应用程序
    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    /// 一般来说update函数只负责CPU侧的逐帧更新逻辑，也就是说，只改变应用状态，而不是提交GPU绘制指令
    fn update(&mut self) {
        // 当前示例只负责清屏，还没有需要逐帧更新的 CPU 状态
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        // 可以把`Surface`想象成窗口和GPU交换链之间的接口，`SurfaceTexture`就是交换链临时提供给你的当前画布
        // `SurfaceTexture`管理当前这一帧和交换链之间的关系，并提供`.present()`进行提交（相当于告诉窗口）系统：
        // 这一帧已经画好了，可以进行显示了。
        //
        // 内部的Texture才是真正绘制的目标
        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(()),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device")
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::wgt::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                // 指定颜色输出目标
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // 方法名称是 set_index_buffer ，而不是 set_index_buffers 。一次只能设置一个索引缓冲区。
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            // 使用索引缓冲区时，你需要使用 draw_indexed 。draw 方法会忽略索引缓冲区。此外，请确保你使用的是索引数量
            // （num_indices）而非顶点数量，否则你的模型要么会绘制错误，要么该方法会因为索引不足而 panic 。
            //
            // ==================== 先记住这四个基础概念 ====================
            //
            // 顶点：形状的点。
            // 索引：告诉 GPU 如何把点组成三角形。
            // 网格：由这些顶点和三角形描述出来的整个形状。
            // 实例：使用同一个形状画出来的其中一份。
            //
            // 简单来说：索引负责“这个形状怎么组成”，实例负责“这个形状画几份”。
            //
            // ============================================================
            //
            // draw_indexed 的三个参数依次是：
            //
            // 1. indices: 0..self.num_indices
            //    指定读取索引缓冲区中的哪些元素。这里的单位是“索引元素”，不是字节；
            //    Rust 的 Range 不包含右端点，因此会读取编号为
            //    0、1、...、num_indices - 1 的全部索引。
            //
            // 2. base_vertex: 0
            //    这是加到每个索引值上的“基础顶点偏移”，单位是“顶点/顶点槽位”，也不是字节。
            //    GPU 从索引缓冲区读出一个索引后，最终访问的顶点编号可理解为：
            //
            //        最终顶点编号 = 索引值 + base_vertex
            //
            //    例如索引缓冲区中的值为 [0, 1, 2]，base_vertex 为 4 时，实际访问的是
            //    顶点 4、5、6。至于这些顶点在 vertex_buffer 中相隔多少字节，则由
            //    VertexBufferLayout::array_stride 决定；例如每个顶点占 24 字节，
            //    顶点 4 相对于顶点缓冲区开头的字节偏移就是 4 * 24 = 96 字节。
            //    base_vertex 的类型是 i32，所以也可以为负数，但计算出的最终顶点编号必须有效。
            //    此处传入 0，表示不修正索引值，索引 0 就访问顶点 0。
            //
            //    这个参数常用于把多个网格的顶点连续放在同一个顶点缓冲区中：每个网格的索引
            //    都可以继续从 0 开始，只需用 base_vertex 指向该网格顶点数据的起始位置。
            //
            // 3. instances: 0..1
            //    指定绘制哪些实例，单位是“实例”，不是字节。0..1 只包含实例编号 0，
            //    所以这里只绘制一个实例；0..3 则会绘制实例 0、1、2，共三个实例。
            //    实例编号可以在着色器中通过 @builtin(instance_index) 取得；如果顶点缓冲区
            //    使用 VertexStepMode::Instance，GPU 也会按照实例而不是按照顶点推进实例数据。
            //
            //    实例与索引是两个相互独立的维度：
            //
            //        索引：决定一次网格绘制要按照什么顺序使用哪些顶点；
            //        实例：决定把这一整套索引绘制过程重复多少次。
            //
            //    它们不是“一个索引对应一个实例”的关系，而是“每个实例都执行一遍完整的
            //    indices 范围”。例如使用 6 个索引并绘制 3 个实例：
            //
            //        draw_indexed(0..6, 0, 0..3)
            //
            //    可以理解为：
            //
            //        实例 0：依次处理索引 0..6
            //        实例 1：依次处理索引 0..6
            //        实例 2：依次处理索引 0..6
            //
            //    因此，顶点着色器的调用次数通常可以理解为：
            //
            //        索引数量 * 实例数量
            //
            //    上面的例子会产生 6 * 3 = 18 次顶点着色器调用（忽略 GPU 可能进行的缓存
            //    与内部优化）。每次调用都同时具有一个“索引最终对应的顶点”和一个实例编号。
            //
            //    如果各实例没有不同的实例数据或着色器变换，它们会绘制到同一位置，看起来
            //    完全重叠。实际的实例化绘制通常会根据 instance_index 读取每个实例各自的
            //    模型矩阵、位置、颜色等数据，让同一个顶点缓冲区和索引缓冲区生成多个外观
            //    或位置不同的物体。实例范围本身不会改变索引指向的顶点。
            //
            // 因此，这次调用的整体含义是：读取全部索引、不添加顶点偏移，并绘制一个实例。
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

            // 在填充三角形之上绘制外轮廓和内部三角形边线。
            render_pass.set_pipeline(&self.line_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass
                .set_index_buffer(self.line_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_line_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    // 这里之所以使用Option，是应为State::new需要窗口，但是应用程序必须进入Resumed状态之后才能创建窗口
    // 补充：也不完全是，因为State的计算过程是异步的，这里一开始就是要填成Option
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        Self {
            #[cfg(target_arch = "wasm32")]
            proxy: Some(event_loop.create_proxy()),
            state: None,
        }
    }
}

// 关于`EventLoop`和`ActiveEventLoop`之间的区别的理解，EventLoop的作用是创建并拥有事件循环，配置它，然后启动它
// 但是`ActiveEventLoop`的作用是在事件循环运行期间，winit传给回调的“当前活动上下文“

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut state = pollster::block_on(State::new(window)).unwrap();
            let size = state.window.inner_size();
            state.resize(size.width, size.height);
            state.window.request_redraw();
            self.state = Some(state);
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(err) => {
                        tracing::error!(?err);
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => state.handle_key(event_loop, code, key_state.is_pressed()),
            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        let size = event.window.inner_size();
        event.resize(size.width, size.height);
        event.window.request_redraw();
        self.state = Some(event);
    }
}

// 原生 bin 和浏览器都调用同一个入口；平台差异只保留在 run 函数内部。
// WASM 下 wasm-bindgen 会在 JavaScript 完成模块初始化后自动调用它。
#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn run() {
    #[cfg(not(target_arch = "wasm32"))]
    {
        use winit::event_loop::EventLoop;

        env_logger::init();

        let event_loop = EventLoop::with_user_event()
            .build()
            .expect("failed to create event loop");
        let mut app = App::new();
        event_loop
            .run_app(&mut app)
            .expect("failed to run application");
    }

    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        console_log::init_with_level(log::Level::Info).unwrap_throw();

        let event_loop = EventLoop::with_user_event().build().unwrap_throw();
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }
}
