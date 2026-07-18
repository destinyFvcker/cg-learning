//! 根据运行平台初始化Rust的全局日志系统，并允许外部调整日志级别

use tracing_subscriber::EnvFilter;

/// - 默认记录 info、warn、error
/// - wgpu_core、wgpu_hal和naga模块都只记录error
/// - debug 和 tracing 都被 info 过滤
const DEFAULT_FILTER: &str = "info,wgpu_core=error,wgpu_hal=error,naga=error";

/// 根据当前平台初始化全局 tracing subscriber。
///
/// 默认记录 `info` 及以上级别，并将输出量较大的 wgpu 和 naga 模块限制为
/// `error`。原生平台可通过 `RUST_LOG` 环境变量覆盖过滤规则；Web 平台则可使用
/// URL 查询参数（例如 `?RUST_LOG=debug`）。
pub fn init_tracing() {
    std::cfg_select! {
        target_arch = "wasm32" => {
            tracing_subscriber::fmt()
                .with_env_filter(web_env_filter())
                .with_ansi(false)
                .without_time()
                .with_writer(tracing_web::MakeWebConsoleWriter::new())
                .init();

            // 安装panic hook，在浏览器之中发生panic的时候，会在Console之中显示更友好的错误信息和调用栈
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
        }
        target_os = "android" => {
            init_native_tracing();
            log_panics::init();
        }
        _ => {
            init_native_tracing();
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn init_native_tracing() {
    let rust_log = std::env::var(EnvFilter::DEFAULT_ENV).ok();

    tracing_subscriber::fmt()
        .with_env_filter(env_filter(rust_log.as_deref()))
        .init();
}

#[cfg(target_arch = "wasm32")]
fn web_env_filter() -> EnvFilter {
    let rust_log = web_sys::window()
        .and_then(|window| window.location().search().ok())
        .and_then(|query| web_sys::UrlSearchParams::new_with_str(&query).ok())
        .and_then(|params| params.get(EnvFilter::DEFAULT_ENV));

    env_filter(rust_log.as_deref())
}

fn env_filter(rust_log: Option<&str>) -> EnvFilter {
    rust_log
        .filter(|directives| !directives.trim().is_empty())
        .map(EnvFilter::new)
        .unwrap_or_else(|| EnvFilter::new(DEFAULT_FILTER))
}
