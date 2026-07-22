// 原生目标始终包含教程模块；只有 WASM 目标需要用 feature 选择唯一的启动模块。
#[cfg(any(not(target_arch = "wasm32"), feature = "buffers-and-indices"))]
pub mod buffers_and_indices;
#[cfg(any(not(target_arch = "wasm32"), feature = "the-window-ch"))]
pub mod the_pipeline;
#[cfg(any(not(target_arch = "wasm32"), feature = "the-surface"))]
pub mod the_surface;
#[cfg(any(not(target_arch = "wasm32"), feature = "the-window"))]
pub mod the_window;
#[cfg(any(not(target_arch = "wasm32"), feature = "the-window-ch"))]
pub mod the_window_ch;
