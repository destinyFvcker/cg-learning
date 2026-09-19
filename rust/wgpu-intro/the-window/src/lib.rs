//! 依赖与窗口，在Rust之中，无论使用什么样的窗口解决方案，都需要实现`raw-window-handle`里面定义的`HasWindowHandle`以及
//! `HasDisplayHandle`这两个抽象接口。

use std::sync::Arc;

use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

pub struct State {
    window: Arc<Window>,
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        Ok(Self { window })
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        todo!()
    }

    pub fn render(&mut self) {
        self.window.request_redraw();
    }
}

// 现在我们有了`State`结构体，需要告诉winit应该如何使用它。我们将会创建一个App结构体来实现

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    // 这里之所以使用Option，是应为State::new需要窗口，但是应用程序必须进入Resumed状态之后才能创建窗口
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        // 下面这个变量仅在Web环境下需要，原因在于创建一个WGPU资源是一个异步过程
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            #[cfg(target_arch = "wasm32")]
            proxy,
            state: None,
        }
    }
}

impl ApplicationHandler<State> for App {
    // resumed 方法看似功能很多，但是实际上就只做这几件事：
    // 1. 定义了窗口的相关属性，包括一些特定于网络的内容
    // 2. 使用这些属性来创建窗口
    // 3. 创建一个future，它会生成State结构体
    // 4. 在原生环境之中，使用pollster来等待future返回的异步结果
    // 5, 在web上，异步运行future，并将结果发送到use_event函数
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
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
            self.state = Some(pollster::block_on(State::new(window)).unwrap());
        }

        // resumed()
        //   │
        //   ├─ 创建 Window
        //   │
        //   ├─ 启动异步 State::new(window)
        //   │       │
        //   │       └─ 完成后得到 State
        //   │
        //   └─ proxy.send_event(State)
        //           │
        //           └─ 事件循环调用 user_event(..., State)
        //                          │
        //                          └─ self.state = Some(State)
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                // 这不是创建一个新线程，只是把这个Future注册到浏览器的异步任务队列之中，让它稍后被执行
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        // 这里发送成功之后，winit就会在事件循环之中调用它，也就是`user_event`方法
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

    // 这里可以处理键盘输入、鼠标移动等事件，以及窗口需要绘制或者调整大小等其他窗口事件。
    // 可以在这里调用在State上定义的相关方法。
    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
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
                state.render();
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } => match (code, key_state.is_pressed()) {
                (KeyCode::Escape, true) => event_loop.exit(),
                _ => {}
            },
            _ => {}
        }
    }

    // user_event函数作为我们State future的着陆点（landing point）,因为resumed不是异步的，所以需要offload future并将
    // 结果发送到某个个地方
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, mut event: State) {
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event)
    }
}

pub fn run() -> anyhow::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    // 可以把EventLoop理解为“操作系统事件循环的跨平台适配层”，但它不只是事件流转换器，更准确说是：
    // EventLoop 是应用主线程上的平台事件泵、窗口生命周期协调器和回调调度器。
    //
    // 它主要负责：
    // 接管/驱动操作系统事件泵，例如 Win32 message loop、macOS NSApplication、Wayland/X11 事件队列。
    // 把平台事件统一转换为 WindowEvent、DeviceEvent 等 winit 事件。
    // 按顺序调用 ApplicationHandler 的回调。
    // 管理应用生命周期，如 resumed、suspended、exiting。
    // 提供窗口创建所需的平台上下文，即 ActiveEventLoop::create_window()。
    // 决定线程何时休眠或继续运行，即 ControlFlow::{Wait, Poll, WaitUntil}。
    // 接收 EventLoopProxy 从其他线程送来的自定义事件。
    // 协调重绘请求，例如把 Window::request_redraw() 转化为 RedrawRequested。

    let event_loop = EventLoop::with_user_event().build()?;
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app)?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}
