// 原生桌面入口只负责选择教程模块；平台初始化统一由模块中的 run 函数处理。
#[cfg(not(target_arch = "wasm32"))]
fn main() {
    wgpu_intro::the_surface::run();
}

// WASM 使用 `the_surface::run` 上的 `wasm_bindgen(start)`，不从 bin 启动。
#[cfg(target_arch = "wasm32")]
fn main() {}
