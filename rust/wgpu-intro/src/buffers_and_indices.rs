//! 缓冲区（Buffer）是GPU上的一块数据。缓冲区保证是连续的，这意味着所有的数据都将会按照顺序存储在内存之中。
//! 缓冲区通常用于存储结构体或数组等简单内容，但是也可以存储更加复杂的内容，例如树等图结构（前提是所有节点都存储在一起，
//! 且不引用缓冲区之外的任何内容）。
//!
//! 我们将大量使用缓冲区，所以让我们从最重要的两点开始：顶点缓冲区（Vertex Buffer）和索引缓冲区（Index Buffer）

// 之前我们直接在顶点着色器之中存储顶点类数据。这虽然在起步阶段运行良好，但是从长远上来看并不可行。
// 我们需要绘制的对象类型在规模上各不相同，而且，每当需要更新模型时都要重新编译着色器会极大降低程序的运行速度。

use std::sync::Arc;

use wgpu::{VertexBufferLayout, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    /// 位置代表顶点在三维空间中的x、y和z坐标
    position: [f32; 3],
    /// 颜色就是顶点的红绿蓝数值
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3];

    fn desc() -> VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            // ` array_stride ` 定义了顶点的宽度。当着色器读取下一个顶点时，
            // 它将跳过 ` array_stride ` 个字节。在我们的案例中，`array_stride` 可能为 24 字节
            //
            // 专门设计这个参数，是因为顶点数据可能包含：内存对齐产生的填充字节、当前不提供给着色器的字段
            // 为方便访问预留的数据、大于属性实际占用范围的步长.
            //
            // [position 12B][color 12B][padding 8B]
            // <--------- array_stride = 32 --------->
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            // VertexStepMode::Vertex 每处理一个顶点，缓冲区就前进一个 array_stride
            // VertexStepMode::Instance GPU 会在开始绘制下一个实例时才推进缓冲区
            step_mode: wgpu::VertexStepMode::Vertex,
            // 顶点属性描述了顶点的各个组成部分--每个顶点包含哪些着色器输入。通常，
            // 这与结构体的字段是一一对应的，在我们的案例中也是如此
            attributes: &[
                wgpu::VertexAttribute {
                    // 从 offset 指向的位置开始，应当读取多少字节，并如何解释这些字节。
                    //
                    // Float32x3 对应着色器代码中的 vec3<f32> 我们可以在一个属性中存储的最大值是
                    // Float32x4（ Uint32x4 和 Sint32x4 同样适用）。当我们需要存储大于
                    // Float32x4 的内容时，我们需要记住这一点。
                    format: wgpu::VertexFormat::Float32x3,
                    // 相对于当前元素起点，这个属性从第几个字节开始。注意是“相对于当前顶点起点”，不是相对于整个 Buffer 起点。
                    offset: 0,
                    // 这会告知着色器将该属性存储在哪个位置。例如，顶点着色器中的 @location(0) x: vec3<f32>
                    // 将对应于 Vertex 结构体中的 position 字段，而 @location(1) x: vec3<f32> 则对应于
                    // color 字段。
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                },
            ],
        }
    }

    // 可以使用`wgpu::vertex_attr_array!`宏来简化当前的顶点格式设置操作操作
    fn desc_macro() -> VertexBufferLayout<'static> {
        use std::mem;

        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
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
    num_vertices: u32,
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

        // 根据 Window / Display 创建 Surface
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        // 选择能够向这个 Surface 呈现画面的 GPU
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

        // 这里实际上在查询 Surface 和 Adapter 的共同能力
        let surface_cap = surface.get_capabilities(&adapter);
        let surface_format = surface_cap
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_cap.formats[0]);
        // 在上面使用insface创建Surface的时候实际上就只做了一件事：根据操作系统的窗口句柄，
        // 创建一个可以向该窗口呈现画面的目标
        //
        // 但是实际上此时它就只建立起大概这样的关系：Surface -> 操作系统窗口
        // 现在还不知道:
        // - 最终使用哪块GPU（Adapter）
        // - 使用哪个逻辑设备（Device）
        // - GPU和窗口共同支持哪些纹理格式
        // - 窗口大小是多少
        // - 使用垂直同步还是即时呈现
        // - Surface纹理的用途是什么
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
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
                // 列表中的每个条目代表一个可以绑定缓冲区的插槽。使用None表示该插槽应该为空。
                // 如果你希望将特定的缓冲区放在特定的插槽之中这就会有用
                //
                // 对于现在的这个演示，将只使用一个缓冲区。即便如此，我们仍然需要在渲染方法之中实际
                // 设置顶点缓冲区，否则程序将会崩溃
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
                    // 可以在这里注意到实际上`config.format`同时出现在`SurfaceConfiguration`和
                    // 这里，也就是`ColorTargetState`之中，这两边必须一致，因为一个描述“目标纹理实
                    // 际是什么格式”，另一个描述“管线认为自己正在向什么格式写”。可以类比为：
                    //
                    // Surface：我提供 BGRA8 sRGB 画布
                    // Pipeline：我将向 BGRA8 sRGB 画布输出
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

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            is_surface_configured: false,
            num_vertices: VERTICES.len() as u32,
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

    /// render则是要通过当前的状态绘制一帧
    fn render(&mut self) -> anyhow::Result<()> {
        // 下面这个调用本身不会绘制，而只是向事件循环申请一次`RedrawRequested`.
        // 由于现在每次在render之中都会调用一次它，每次完成一帧的时候都会预约下一帧，由此形成连续渲染循环
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
            // 成功获取到一张用于绘制当前帧的纹理。纹理格式、尺寸以及Surface状态都正常，可以直接创建TextureView并进行渲染
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            // 成功拿到了纹理，所以当前这一帧依旧可画；但是当前Surface配置已经不完全匹配底层窗口系统了
            // - 窗口尺寸发生了变化
            // - HiDPI缩放比例发生了变化
            // - 窗口从一个显示器移动到了另外一个显示器
            // - 底层交换链属性发生变化
            // - 平台认为当前配置仍然能工作，但已经不是最佳配置
            // 这个时候应该先渲染，然后重新配置对应的Surface比较好
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            // 等待下一张 SurfaceTexture 时超时了，可能的原因包括：
            // - GPU 暂时太忙；
            // - 前面的帧还没有执行完；
            // - 操作系统暂时无法提供新的交换链图像；
            // - 驱动或窗口系统出现短暂延迟。
            wgpu::CurrentSurfaceTexture::Timeout => return Ok(()),
            // 窗口目前不可见或被遮挡，系统没有提供当前帧纹理，典型情况包括：
            // - 窗口最小化；
            // - 窗口完全被其他窗口遮挡；
            // - 窗口当前不在可见桌面；
            // - 某些平台暂停了不可见窗口的交换链。
            wgpu::CurrentSurfaceTexture::Occluded => return Ok(()),
            // Surface 还存在，但之前的 SurfaceConfiguration 已经过期，不能继续使用，常见原因：
            // - 窗口刚刚调整了大小；
            // - 平台交换链发生变化；
            // - 当前配置中的宽高不再匹配窗口；
            // - 显示环境发生改变。
            //
            // - 它与 Suboptimal 的核心区别是：
            // - Suboptimal：纹理还能用，这一帧可以继续画；
            // - Outdated：纹理都没有拿到，这一帧画不了。
            // 一般不需要在同一个 render() 调用里立刻重试。重新配置后结束这一帧，下一次重绘再获取纹理，逻辑会更简单。
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            // 含义比 Outdated 严重：
            // Surface 本身已经丢失，单纯重新调用 configure() 不一定能恢复。
            //
            // 这里要区分两层资源：
            // Instance
            //   └── Surface
            //         └── 当前交换链纹理
            // Outdated 通常只是最后一层交换链配置失效；Lost 表示 Surface 与底层窗口表面的连接可能已经失效。
            // 官方建议是：
            // 使用 Instance::create_surface() 重新创建 Surface；
            // 调用 Surface::configure()；
            // 下一帧重试。
            //
            // 而且假如整个gpu device都丢失了，那这里甚至要整个状态全部进行重新构建
            wgpu::CurrentSurfaceTexture::Lost => {
                anyhow::bail!("Lost device")
            }
            // get_current_texture() 内部产生了 wgpu 验证错误，并且这个错误被 error scope 或未捕获错误回调捕获了。
            // 它通常意味着程序的使用方式存在问题，而不是普通的运行时波动，可能原因包括：
            // - Surface 尚未正确 configure()；
            // - SurfaceConfiguration 不受支持；
            // - 配置尺寸不合法；
            // - 资源生命周期或 API 调用顺序错误；
            // - 底层状态与程序记录的状态不一致。
            wgpu::CurrentSurfaceTexture::Validation => return Ok(()),
        };

        // RenderPass 通过 TextureView 写入 Texture
        // TextureView 表示GPU在本次渲染之中"如何访问这张Texture"的视图
        let view = output
            .texture
            .create_view(&wgpu::wgt::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            // RenderPass可以理解成：一次针对一组渲染附件（Render Attachments）的绘制阶段
            // 这里的附件通常包括：
            // - 一个或多个颜色附件，例如窗口交换链纹理
            // - 可选的深度附件
            // - 可选的模板附件
            //
            // 1️⃣ 绘制目标
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

            // 2️⃣ 绘制状态
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            // 3️⃣ 绘制命令：我们告诉 wgpu 使用三个顶点和一个实例来绘制内容。
            // 这就是 @builtin(vertex_index) 的来源
            render_pass.draw(0..self.num_vertices, 0..1);
        }

        // 结束 render pass 只代表命令已被记录；还需要提交给 GPU，并将这一帧呈现到窗口。
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
