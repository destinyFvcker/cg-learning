// 原生桌面入口只负责运行本教程模块；平台初始化统一由 lib 中的 run 函数处理。
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    uniform_buffers_and_3d_camera::run();
}

// WASM 使用 lib 中 `run` 上的 `wasm_bindgen(start)`，不从 bin 启动。
#[cfg(target_arch = "wasm32")]
fn main() {}
