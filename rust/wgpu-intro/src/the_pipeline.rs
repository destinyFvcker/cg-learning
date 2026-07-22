#![allow(unused)]

//! 如果你熟悉OpenGL，可能会记得使用着色器程序。你可以将这个所谓的管线视为其更加强大的版本。
//! 管线描述了GPU在处理一组数据时将执行的所有操作。

// WebGPU对几何着色器和细分着色器的支持并不完善，所以应该尽量避免使用它们

// 顶点是三维空间（也可以说是二维空间）之中的一个点。这些顶点随后被成对组合形成线段，或三个一组形成三角形
// 我们使用顶点着色器来操控顶点，从而将形状变换成我们想要的样子。
// 顶点随后被转换为片段。结果图像中的每个像素至少对应一个片段。
// 每个片段都带有颜色，该颜色将被复制到其对应的像素上。片段着色器决定了片段最终呈现的颜色。

use std::{iter, sync::Arc};

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    render_pipeline: wgpu::RenderPipeline,
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        // Surface可以理解为：GPU和操作系统窗口之间的“显示接口“，代表的是窗口中那块可以接收并显示GPU渲染结果的区域
        //
        // Window
        //   ↑
        // Surface        把渲染结果提交到窗口
        //   ↑
        // SurfaceTexture 当前这一帧对应的屏幕纹理
        //   ↑
        // RenderPass     GPU 将三角形、模型等画进去
        //
        // 关于为什么需要Surface，因为GPU通常现把画面渲染到一张纹理之中，但是最终还要让操作系统把纹理显示在窗口之中。
        // 这部分在不同的平台之中显示机制不一样。Surface的存在正是要把这些平台差异统一封装起来。
        //
        // 真正被GPU绘制的是每一帧从Surface之中取出来的SurfaceTexture
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
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
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

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            color_space: wgpu::SurfaceColorSpace::Auto,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        // 可以使用incluade_wgsl!宏作为创建此shader的快捷方式
        // let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../../../media/shaders/wgsl/shader.wgsl").into(),
            ),
        });

        // 渲染管线布局描述的是：Shader可以从CPU/GPU外部接收哪些资源和立即数据
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                // 给对象设置调试名称。不会影响渲染结果或性能
                label: Some("Render Pipeline Layout"),
                // 表示这个管线没有声明任何绑定组，因此 Shader 不通过绑定组访问纹理、采样器、
                // Uniform Buffer 或 Storage Buffer 等资源。
                bind_group_layouts: &[],
                // 表示没有为 Shader 分配 immediate data（立即数据）
                immediate_size: 0,
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                // 在这里，你可以指定着色器中的哪个函数作为 entry_point 。
                // 这些就是我们用 @vertex 和 @fragment 标记的函数。
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            // ` primitive ` 字段描述了在将顶点转换为三角形时如何解释这些顶点
            primitive: wgpu::PrimitiveState {
                // 使用 PrimitiveTopology::TriangleList 意味着每三个顶点将对应一个三角形
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                // front_face 和 cull_mode 字段告诉 wgpu 如何判断给定的三角形是否朝前（正面）
                // FrontFace::Ccw 表示如果顶点按逆时针方向排列，则该三角形朝前（正面）
                // 不被视为朝前的三角形将根据 CullMode::Back 的指定被剔除（不包含在渲染中）
                //       v2
                //      /  \
                //    v0 → v1     // v0 → v1 → v2 为逆时针
                front_face: wgpu::FrontFace::Ccw,
                // 启用背面剔除：只绘制正面，丢弃背面。它通常用于封闭的 3D 模型，因为模型内部的表面一般不可见，可以减少无用绘制
                cull_mode: Some(wgpu::Face::Back),
                // 使用正常的深度裁剪，超出深度范围的部分会被裁掉
                unclipped_depth: false,
                // 填充整个三角形。其他模式还可能包括只画边框的 Line 或只画顶点的 Point，但它们可能需要额外的设备特性支持
                polygon_mode: wgpu::PolygonMode::Fill,
                // 关闭保守光栅化, 普通渲染通常保持 false；保守光栅化主要用于碰撞检测、体素化等特殊场景
                conservative: false,
            },
            // 我们目前没有使用深度/模板缓冲（depth/stencil buffer），因此我们将 depth_stencil 保持为 None 。这在以后会有所改变。
            depth_stencil: None,
            // `multisample` 决定了渲染管线将使用多少个样本。多重采样是一个复杂的话题，所以我们在这里不做深入探讨
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            // fragment 在技术上是可选的，因此你需要将其包装在 Some() 中。
            // 如果我们要将颜色数据存储到 surface ，就需要用到它。
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                // `targets` 字段告诉 `wgpu` 应该设置哪些颜色输出。这里只有一个元素，表示管线只有一个颜色输出目标，通常就是窗口的 Surface。
                //
                // 之所以是数组，是因为现代 GPU 支持一次向多个纹理输出，即 MRT（Multiple Render Targets）。
                // 例如延迟渲染中，可以同时输出颜色、法线和材质参数。
                //
                // 我们使用 `surface` 的格式以便于进行复制操作，并指定混合模式应直接用新数据替换旧的像素数据。
                // 我们还告知 `wgpu` 写入所有颜色通道：红、蓝、绿以及 Alpha。在讨论纹理时，我们会进一步探讨 `color_state`。
                targets: &[Some(wgpu::ColorTargetState {
                    // 它必须和 Render Pass 中对应颜色附件的格式一致
                    format: config.format,
                    // 指定新颜色和目标中已有颜色如何混合，REPLACE表示直接使用新的颜色
                    blend: Some(wgpu::BlendState::REPLACE),
                    // 决定允许写入哪些颜色通道。ALL 表示写入所有通道
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            // multiview_mask 表示渲染附件可以拥有的数组层数。我们不会渲染到数组纹理，因此可以将其设置为 None
            multiview_mask: None,
            // cache 允许 wgpu 缓存着色器编译数据, 这仅对 Android 构建目标真正有用
            cache: None,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            render_pipeline,
            window,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    fn update(&mut self) {
        // 当前示例只负责清屏，还咩有需要逐帧更新的 CPU 状态
    }

    fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                anyhow::bail!("Lost device");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
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
            // 我们告诉 wgpu 使用三个顶点和一个实例来绘制内容。这就是 @builtin(vertex_index) 的来源
            render_pass.draw(0..3, 0..1);
        }

        // 结束 render pass 只代表命令已被记录；还需要提交给 GPU，并将这一帧呈现到窗口。
        self.queue.submit(iter::once(encoder.finish()));
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
