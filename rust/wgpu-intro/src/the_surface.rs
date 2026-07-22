#![allow(unused)]

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<State> {
        let size = window.inner_size();

        // instance是使用wgpu时创建的第一个对象，提供了若干用于开始访问系统GPU的操作入口。
        // 其主要用途是创建Adapter和Surface
        // Instance
        //      └─ request_adapter() → Adapter（选择物理 GPU + 图形后端）
        //              └─ request_device() → Device + Queue（逻辑设备）
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            // backends：指定允许wgpu尝试的图形api集合，而不是立即选定某一个API
            //
            // PRIMARY代表的是平台原生的图形API，Windows上是DirectX12或者Vulkan，Linux上只对应Vulkan
            // macos/ios上是Metal，而浏览器上就是Browser WebGPU
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            // Backends::GL被wgpu列为secondary backend，主要对应原生平台的OpenGL ES/EGL/ANGLE，
            // 以及WASM浏览器之中的WebGL2
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            // 控制调试和验证行为，就像是：
            // - 是否生成调试信息
            // - 是否开启Vulkan Validation Layer
            // - 是否开启D3D12 Debug layer
            // - 是否验证indirect draw/dispatch 参数
            // - 是否允许不完全合规的驱动
            flags: Default::default(),
            // 控制GPU显存压力达到什么程度的时候，提前拒绝资源创建或让设备丢失
            memory_budget_thresholds: Default::default(),
            // 各个图形后端专属的配置结合，例如：
            // - GL/GLES 请求哪个 GLES 3.x 版本
            // - GL fence 的行为
            // - 是否启用GL调试函数
            // - DX12 shader compiler 和 Agility SDK 配置
            // - noop测试后端配置
            backend_options: Default::default(),
            // 操作系统display/compositor的链接，就像是：
            // - Wayland display
            // - X11 display
            // - Windows display/window 系统链接
            // - macos diaplay相关句柄
            // None表示创建Instance的时候不预先绑定display，后面调用create_surface(window)的时候再从窗口对象获取相关句柄
            //
            // 如果这里传入某个个display handle，那么以后创建surface的时候就不能传入属于另一个display的窗口。
            //
            // 操作系统窗口系统 / compositor
            //         │
            //         ▼
            // Display（连接/上下文）
            //         │
            //         ├── Window A
            //         ├── Window B
            //         └── Window C
            //                │
            //                ▼
            //         wgpu::Surface
            //                │
            //                ▼
            //         GPU 渲染结果显示到窗口
            display: None,
        });

        // surface是绘制到的窗口部分，需要它来直接绘制到屏幕
        let surface = instance.create_surface(window.clone()).unwrap();

        // adapter是我们实际显卡的句柄。可以用它来获取显卡信息，例如显卡名称以及适配器使用的后端。是图形领域之中的术语，
        // 表示一个物理或虚拟GPU的抽象句柄
        //
        // Instance
        // ├── Intel 集成显卡 Adapter
        // ├── NVIDIA 独立显卡 Adapter
        // └── 软件渲染 Adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                // 有三种变体：None、LowPower和HighPerformance，选择偏向电池续航的适配器，还是选择功耗很高但性能更强的GPU适配器
                power_preference: wgpu::PowerPreference::default(),
                // 告诉wgpu寻找一个能够向提供的Surface呈现内容的适配器
                compatible_surface: Some(&surface),
                // 强制wgpu选择一个能在所有硬件上运行的适配器。这通常意味着渲染后端将使用"软件"系统，而非GPU等硬件
                force_fallback_adapter: false,
                // 报告实际能力限制，而不是把限制值归入预设档位。档位化主要用于减少硬件指纹识别
                apply_limit_buckets: false,
            })
            .await?;
        // 如果想要获取特定后端的所有适配器，可以使用enumerate_adapters，这将会返回一个迭代器，可以遍历它来检查是否有
        // 适配器满足你的需求（但是这在WASM上不可用，只能使用request_adapter）
        //
        // 实际上Adapter是候选GPU的描述和能力信息，就像是：
        // - 是独立显卡还是集成显卡
        // - 支持哪些功能
        // - 最大纹理尺寸是多少
        // - 能否向当前窗口显示画面

        // 已经选好GPU（Adapter）之后，向它申请一个真正可用于绘图的逻辑设备Device，以及向GPU提交工作的队列Queue
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                // 给逻辑设备附加调试名称，不会影响功能或性能
                label: None,
                // 声明程序要求启用哪些可选的GPU功能，这里选择不启用
                required_features: wgpu::Features::empty(),
                // 禁止使用仍在开发中的实验性特性
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // 声明程序需要的GPU能力下限和将被允许使用的上限，就像是：
                // - 最大纹理尺寸
                // - 最大缓冲区大小
                // - 最大绑定数量
                // - Compute Shader工作组大小
                required_limits: if cfg!(target_arch = "wasm32") {
                    // 对于WASM使用更加保守的WebGL2兼容限制，因为WebGL2能力非常受限
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::defaults()
                },
                // 告诉wgpu更倾向于怎样管理GPU内存
                memory_hints: Default::default(),
                // 控制是否记录wgpu API调用轨迹，Off表示不记录，开启之后主要用于调试和复现底层图形问题。
                // 不是图形日志功能
                trace: wgpu::Trace::Off,
            })
            .await?;

        // 为窗口的绘制表面Surface选择合适的颜色格式，并创建渲染配置。
        //
        // 查询当前GPU适配器与窗口表面组合支持的能力，包括：
        // - formats: 支持的像素/颜色格式
        // - persent_modes: 画面如何提交到显示器
        // - alpha_modes: 窗口透明度如何合成
        // - usages: 表面纹理支持的用途
        let surface_caps = surface.get_capabilities(&adapter);
        // 选择sRGB格式，如果找不到sRGB，就使用第一个支持的格式
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        // 描述窗口表面应该如何产生每一帧用于渲染的SurfaceTexture
        let config = wgpu::SurfaceConfiguration {
            // 表示表面纹理会作为渲染目标使用，也就是 fragment shader 最终把颜色写到这里
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            // 使用刚才选出的颜色格式。渲染管线的颜色目标格式通常也必须与它匹配
            // format：像素在显存中怎么存，例如 Bgra8UnormSrgb
            format: surface_format,
            // olor_space：显示系统应该怎样解释这些颜色，例如 sRGB、Display P3、HDR10
            color_space: wgpu::SurfaceColorSpace::Auto,
            // 设置表面纹理尺寸，通常等于窗口客户区的像素尺寸
            // 确保 SurfaceTexture 的宽度和高度不为 0，否则可能导致应用崩溃。
            width: size.width,
            height: size.height,
            // 选择画面呈现模式，改枚举决定了如何将Surface和显示器同步。
            // 不同模式决定是否等待垂直同步以及如何处理来不及显示的帧，例如：
            // Fifo：类似 VSync(垂直同步)，不撕裂，所有平台保证支持
            // Immediate：立即呈现，延迟低，但可能画面撕裂
            // Mailbox：保留最新帧，通常低延迟且不撕裂
            present_mode: surface_caps.present_modes[0],
            // 希望从获得当前 Surface 纹理到画面显示之间，最多约有 2 次显示器刷新。
            // 它主要用于在吞吐量和输入延迟之间取得平衡，而且只是给图形后端的提示。
            desired_maximum_frame_latency: 2,
            // 规定窗口表面中的 alpha 通道如何和桌面背景合成。普通不透明窗口通常最终使用不透明模式。
            alpha_mode: surface_caps.alpha_modes[0],
            // 不允许额外的纹理视图格式。表面原本的 format 始终可以使用。
            view_formats: vec![],
        };

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
        })
    }

    // 如果希望在应用程序之中支持调整大小，就需要在窗口尺寸每次变化的时候重新配置surface
    // 这里有一点需要注意：WebGL之中支持的最大尺寸是2048像素，如果显示器分辨率更高，就需要限制高度和宽度
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.is_surface_configured = true;
        }
    }

    // 处理键盘实践，这里只是在按下退出键的时候退出应用程序
    fn handle_key(&self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        match (code, is_pressed) {
            (KeyCode::Escape, true) => event_loop.exit(),
            _ => {}
        }
    }

    fn update(&mut self) {
        // 当前示例只负责清屏，还没有需要逐帧更新的 CPU 状态。
    }

    // 接下来就是重头戏了：首先，我们获取一个用于渲染的帧
    fn render(&mut self) -> anyhow::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        // get_current_texture 函数将等待 surface 提供一个新的 SurfaceTexture ，
        // 我们将渲染到这个 SurfaceTexture 上。我们会将其存储在 output 中供后续使用。
        //
        // 获取 SurfaceTexture
        //         ↓
        // 创建 TextureView
        //         ↓
        // 编码 RenderPass / 绘制命令
        //         ↓
        // 提交 GPU 命令
        //         ↓
        // present() 显示到窗口
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

        // 这行代码使用默认设置创建了一个 TextureView 。我们需要这样做，因为我们要控制渲染代码如何与纹理交互。
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        // 还需要创建一个CommandEncoder来生成实际发送给GPU的命令。大多数现代图形框架要求命令在发送到GPU之前
        // 先存储在命令缓冲区之中。encoder负责构建一个命令缓冲区，随后我们可以将其发送给GPU
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // 终于，现在我们可以开始清屏了。需要使用 encoder 创建一个 RenderPass，包含这一次渲染通道的附件和操作。
        // 这个额外的代码块用于限制 `_render_pass` 的生命周期：RenderPass 存活期间会独占借用 encoder，
        // 因此必须先在块结尾销毁 RenderPass，后面才能调用 encoder.finish() 完成命令缓冲区。
        {
            // color_attachments 字段是一个“稀疏”数组。这允许你使用需要多个渲染目标的管线，并只提供你关心的那些。
            //
            // begin_render_pass() 在 encoder 中开始记录一个渲染通道，并返回代表“正在记录中的通道”的 RenderPass。
            // 变量名前的下划线表示当前示例只依靠创建和销毁通道完成清屏，没有继续调用 set_pipeline() 或 draw()，
            // 同时可以避免 Rust 对“变量未使用”的警告。
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                // 给渲染通道设置一个只用于调试、错误信息和 GPU 调试工具显示的名称，不影响渲染结果和性能语义。
                label: Some("Render Pass"),
                // 指定这个渲染通道要写入的颜色附件列表。列表位置对应颜色输出槽位；这里仅使用槽位 0。
                // 每个槽位都是 Option，因此可以用 None 留出空槽；这里的 Some 表示槽位 0 确实绑定了一个颜色附件。
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    // 指向本帧 SurfaceTexture 创建出的 TextureView，清屏和后续绘制产生的颜色都会写入这个纹理视图。
                    view: &view,
                    // 仅当 view 是三维纹理视图时，才用它选择要渲染的深度切片。
                    // 当前 Surface 是普通二维交换链纹理，所以不指定切片。
                    depth_slice: None,
                    // 多重采样抗锯齿（MSAA）时，可以把多采样附件解析到一个单采样纹理中。
                    // 当前没有创建多采样颜色附件，因此不需要解析目标。
                    resolve_target: None,
                    // 定义这个颜色附件在渲染通道开始和结束时应执行的加载、保存操作。
                    ops: wgpu::Operations {
                        // LoadOp::Clear 表示通道开始时不保留附件原来的像素，而是用下面给出的颜色覆盖整个附件。
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            // 清屏颜色的红色分量，取值通常位于 0.0 到 1.0；这里使用较低的红色强度。
                            r: 0.1,
                            // 清屏颜色的绿色分量；0.2 让绿色强度略高于红色。
                            g: 0.2,
                            // 清屏颜色的蓝色分量；0.3 是三个颜色分量中最高的，因此最终背景偏深蓝色。
                            b: 0.3,
                            // 清屏颜色的 Alpha 分量；1.0 表示完全不透明。
                            a: 1.0,
                            // 结束 Color 结构体和 LoadOp::Clear 的参数。
                        }),
                        // StoreOp::Store 表示渲染通道结束后保留颜色附件中的最终像素。
                        // Surface 后续需要呈现这些像素，因此这里不能使用 Discard 丢弃结果。
                        store: wgpu::StoreOp::Store,
                        // 结束颜色附件的加载与保存操作配置。
                    },
                    // 结束槽位 0 的 RenderPassColorAttachment，并结束颜色附件切片。
                })],
                // 可选的深度/模板附件用于深度测试、深度写入和模板测试；当前仅执行颜色清屏，所以不需要它。
                depth_stencil_attachment: None,
                // 可在渲染通道开始和结束时向 QuerySet 写入 GPU 时间戳，用于性能测量；当前示例未启用时间戳查询。
                timestamp_writes: None,
                // 遮挡查询可以统计通过深度/模板测试的样本，用于判断物体是否可见；当前没有执行绘制，所以不需要查询集。
                occlusion_query_set: None,
                // 多视图渲染可在一次通道中向纹理数组的多个层渲染，常用于 VR 双眼画面；None 表示普通单视图渲染。
                multiview_mask: None,
                // 结束 RenderPassDescriptor，并完成 begin_render_pass() 调用。
            });
            // 离开这个代码块时 `_render_pass` 被销毁，渲染通道结束，encoder 也重新变为可用状态。
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
