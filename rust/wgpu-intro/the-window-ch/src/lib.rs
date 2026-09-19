//! 中文文档疑似已经很久没有更新过了，里面的都是老内容，以后不会再参考中文稳文档的内容

#![allow(unused)]

use std::sync::Arc;

use parking_lot::Mutex;
use winit::{application::ApplicationHandler, event::WindowEvent, window::Window};

// WgpuApp结构体持有窗口实例，并根据目标平台（桌面或者Web）进行相应的初始化
struct WgpuApp {
    /// 避免窗口被释放
    window: Arc<Window>,
}

impl WgpuApp {
    // `async`关键字主要是为了照顾Web端，通过异步方式来防止阻塞主线程
    async fn new(window: Arc<Window>) -> Self {
        // ...
        Self { window }
    }
}

/// 现在暂时感觉下面的这个`Arc<Mutex>`结构存在一定的误导性，在原文档之中是这么说的:
///
/// `使用 `parking_lot::Mutex` 提供更高效的锁机制，结合 Rc 引用计数，确保 WgpuApp 可以在不同的线程中被实例化。`
///
/// 但是实际上ApplicationHandler的resumed、window_event都是通过&mut进行调用的，实际上已经保证了独占性，而且
/// `EventLoop::run_app`在事件循环线程之中**串行**调用这些回调
///
/// 所以我实际认为这就是一个过度设计，可能作者后面还有什么考量吧。如果将来确实需要从异步任务把结果送回事件循环，
/// 英文教程采用的 EventLoopProxy + user_event 会比共享 Mutex 更符合 winit 的模型。
///
/// 反而上面的这个WgpuApp之中的`window`字段是需要Arc进行保护的，因为后面的wgpu::Surface的时候常用于解决窗口句柄的生命周期问题
#[derive(Default)]
struct WgpuAppHandler {
    app: Arc<Mutex<Option<WgpuApp>>>,
}

// 实现ApplicationHandler trait，处理各种窗口事件，如恢复、暂停、关闭请求、大小调整等等
impl ApplicationHandler for WgpuAppHandler {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // 恢复事件
        if self.app.as_ref().lock().is_some() {
            return;
        }

        let window_attributes = Window::default_attributes().with_title("tutorial1-window");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        let wgpu_app = pollster::block_on(WgpuApp::new(window));
        self.app.lock().replace(wgpu_app);
    }

    fn suspended(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        // 暂停事件
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        // 窗口事件
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_size) => {
                // 窗口大小改变
            }
            WindowEvent::KeyboardInput { .. } => {
                // 键盘事件
            }
            WindowEvent::RedrawRequested => {
                // surface重绘事件
            }
            _ => {}
        }
    }
}

// 接下来应该是可以在入口函数之中运行这些代码的，只用在对应的main函数之中创建EventLoop并调用run_app()运行即可
// fn main() -> Result<(), impl std::error::Error> {
//     utils::init_logger();

//     let events_loop = EventLoop::new().unwrap();
//     let mut app = WgpuAppHandler::default();
//     events_loop.run_app(&mut app)
// }
