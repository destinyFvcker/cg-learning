//! 纹理是覆盖在三角形网格上的图像，用于使其看起来更具细节。纹理有很多种类型，就像是法线贴图、凹凸贴图、
//! 高光贴图和漫反射贴图。这里讨论的就是漫反射贴图，或者更简单地说，就是颜色纹理。

use std::sync::Arc;

use wgpu::VertexBufferLayout;
use winit::window::Window;

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1=> Float32x3];

    fn desc() -> VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

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

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    is_surface_configured: bool,
    window: Arc<Window>,
}

impl State {
    async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        // 初始化 wgpu 的全局运行时/根上下文
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

        // 这一步只把 Surface 连接到操作系统窗口。此时它还不知道使用哪块 GPU、像素格式或呈现模式。
        //
        // Surface因为是交换链，所以实际上是和具体的图形后端绑定的，就像是Vulkan、Metal或者GL可能都有自己需求的交换链
        // 格式，下面的这个adapter也是在确定这个唯一的后端
        let surface = instance.create_surface(Arc::clone(&window)).unwrap();

        // 让 wgpu 枚举当前可用的 GPU/图形后端组合，根据条件过滤、排序，然后选出一个并包装成 Adapter 返回。
        //
        // compatible_surface: Some(&surface) 并不是用 Surface“创建 GPU”，而是从系统的候选 GPU 中选择一个
        // 能够向这个窗口呈现画面的 Adapter。
        //
        // Instance 中启用的图形后端
        // │
        // ├─ Vulkan：枚举可见 GPU
        // ├─ Metal：枚举可见 GPU
        // ├─ DX12：枚举可见 GPU
        // └─ GL：枚举可见 GPU
        //          │
        //          ▼
        //   得到候选 Adapter
        //          │
        //          ├─ 根据 Surface 兼容性过滤
        //          ├─ 根据 fallback 要求过滤
        //          ├─ 根据 wgpu 基础能力过滤
        //          ├─ 根据功耗偏好排序
        //          ▼
        //     选择其中一个
        //          │
        //          ▼
        //   返回 wgpu::Adapter
        //
        // 假设 Windows 笔记本有：
        // - Intel 集成显卡
        // - NVIDIA 独立显卡
        // - 软件渲染器
        // 同时启用了 Vulkan 和 DX12，那么候选项在概念上甚至可能是：
        // - Intel + Vulkan
        // - Intel + DX12
        // - NVIDIA + Vulkan
        // - NVIDIA + DX12
        // 软件渲染器 + Vulkan
        // 所以这里选择的不仅是“哪块显卡”，还包含“通过哪个图形后端访问它”。
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: Default::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
                apply_limit_buckets: false,
            })
            .await?;

        // 在创建逻辑设备的时候，实际上信息来源于两个地方：
        // 1. 首先就是Adapter实际隐含提供硬件实际上能做什么
        // 2. DeviceDescripter明确声明程序希望使用什么
        let (device, queue) = adapter
            .request_device(&wgpu::wgt::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                // 下面这个参数会和实际上硬件支持的相关limit进行比较，如果超出了硬件支持的limit就会报错
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

        // 这里也没有再次创建 Surface，而是在查询二者的共同能力。例如，同一个 Surface 面对 Metal、Vulkan
        // 或不同 GPU 时，支持的颜色格式和呈现模式可能不同。
        let surface_caps = surface.get_capabilities(&adapter);

        // 系统：我支持这些格式
        // 应用：那我优先选择其中的 sRGB 格式
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

        let shader = device.create_shader_module(wgpu::include_wgsl!(
            "../media/shaders/buffer-indices-shader.wgsl"
        ));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline"),
                bind_group_layouts: &[],
                immediate_size: 0,
            });

        todo!()
    }
}
