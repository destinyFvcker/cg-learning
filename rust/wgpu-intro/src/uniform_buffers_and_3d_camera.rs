//! 纹理是覆盖在三角形网格上的图像，用于使其看起来更具细节。纹理有很多种类型，就像是法线贴图、凹凸贴图、
//! 高光贴图和漫反射贴图。这里讨论的就是漫反射贴图，或者更简单地说，就是颜色纹理
//!
//! BindGroupLayout
//!     描述一个绑定组内部有哪些资源，以及它们位于哪个 binding
//!
//! PipelineLayout
//!     描述整个渲染管线会使用哪些 BindGroupLayout
//!
//! VertexBufferLayout
//!     描述顶点缓冲区中顶点数据的排列方式
//!
//! PipelineLayout
//! ├── BindGroupLayout（group 0）
//! │   ├── binding 0: texture
//! │   └── binding 1: sampler
//! └── BindGroupLayout（group 1）
//!     └── binding 0: uniform buffer

#![allow(unused)]

use std::{os::macos::raw::stat, sync::Arc};

use wgpu::{VertexBufferLayout, util::DeviceExt};
use winit::{
    application::ApplicationHandler,
    event::{KeyEvent, WindowEvent},
    event_loop::ActiveEventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

use crate::abstracts::texture;

// ============================================================================
// 为什么需要 OPENGL_TO_WGPU_MATRIX：坐标系深度约定转换
// ----------------------------------------------------------------------------
//
// 一、问题本质：OpenGL 与 WebGPU 的 clip-space 深度范围不同
//
// WebGPU 的 clip volume 要求深度满足 0 <= z_c <= w_c（除以 w 后对应
// NDC 的 z ∈ [0,1]）；而 cgmath::perspective() 是 OpenGL 风格的投影矩阵
// （等价于 gluPerspective），其设计目标是 z_NDC ∈ [-1, 1]。两者存在约定
// 不匹配，这正是这个矩阵存在的意义：修正投影结果与 WebGPU 深度范围定义
// 之间的差异，而不是"把模型从原点推开"。
//
// 二、情况 1：不做任何 projection/view 变换
//
// 如果 vertex shader 直接把模型坐标输出：
//     out.position = vec4<f32>(position, 1.0);
// 那么模型坐标会被直接当作 clip-space 坐标，此时 w = 1。一个以 z = 0
// 为中心的模型（例如 z ∈ [-0.5, 0.5] 的立方体）：
//
//     z = +0.5   ✅
//     z =  0     ✅ WebGPU near clipping plane
//     z = -0.5   ❌ 落在 clip volume 之外
//
// WebGPU clip volume:
//       z = 1
//         │
//         │   合法
//         │
// z = 0 ──┼──────── near boundary
//         │
//         │   非法
//         │
//       z < 0
//
// 即教程所说的 "models centered on (0, 0, 0) will be halfway inside the
// clipping area"——负 Z 的一半被裁掉。
//
// 此时乘上 OPENGL_TO_WGPU_MATRIX，它做的是 z'_c = 0.5 * z_c + 0.5 * w_c；
// 因为 w = 1，所以 z' = 0.5z + 0.5，把 [-1, 1] 映射到 [0, 1]。例如原来
// z ∈ [-0.5, 0.5]，转换后 z' ∈ [0.25, 0.75]，整个立方体都进入合法区域。
//
// 三、情况 2：使用 camera + cgmath::perspective（本文件的情况）
//
// model → view → eye space → cgmath::perspective → OpenGL-style clip space
//     → OPENGL_TO_WGPU_MATRIX → WebGPU-style clip space → clipping → ÷w → NDC
//
// 注意：有 camera matrix ≠ 不需要这个转换矩阵。即使相机（如 eye =
// (0,1,2)）已经把模型推到观察空间深处、很多顶点 z_c > 0，删掉转换后
// "看起来能正常显示"，但数学约定仍然是错的：cgmath::perspective 认为
// near → NDC z = -1，而 WebGPU 会把 z_c < 0 的部分全部裁掉：
//
//     OpenGL NDC:  -1 -------- 0 -------- +1
//                   ↑                    ↑
//                  near                 far
//                  左半段全部不符合 WebGPU 的深度 clip volume
//
// 于是你设置的真正 near 不再是 WebGPU 实际使用的裁剪位置。对普通透视
// 矩阵，z_ndc = 0 大约出现在 z ≈ 2nf/(n+f)。代入 n=0.1, f=100 得到
// 约 0.1998，即你告诉 cgmath 近平面是 0.1，不做转换时 WebGPU 实际要到
// 约 0.2 才开始接受：
//
//     camera
//      │
//      │ 0.1 ← cgmath 认为的 near
//      │     ❌ 这一段仍可能被 WebGPU 裁掉
//      │ 0.2 ← 大约到这里 WebGPU 才接受
//      ▼
//     scene
//
// 结论：把 OPENGL_TO_WGPU_MATRIX 理解为深度范围约定修正（[-1,1] → [0,1]）。
// 教程 Note 里"没有 camera matrix 才需要它"的表述不够严谨——删掉它有时
// 仍能看到模型，不代表约定正确。
//
// 四、延伸：为什么 WebGPU 要专门把深度"压缩"成 [0,1]？
//
// 1. 直接原因：跟随现代原生 API
//
// WebGPU 本身不做光栅化，它是 Vulkan / Metal / D3D12 之上的一层薄封装，
// 而这三个底层 API 全部使用 z ∈ [0,1]（D3D 从 D3D10 起、Metal、Vulkan
// 均是如此），WebGPU 原样透传即可。真正的问题是：为什么现代 API 都抛弃
// 了 OpenGL 的 [-1,1]。
//
// 2. [-1,1] 是历史包袱
//
// OpenGL 的 [-1,1] 来自上世纪 80 年代的 SGI Iris GL，设计哲学是三个轴
// 统一成对称区间，数学上"优雅"。但深度与 x、y 的物理性质不同：x、y 是
// 真正的双向量（物体可在中心任一侧），而深度是从相机出发、朝一个方向
// 度量的距离，可见点的"距离"永远是非负量。
//
// 注意这里的一个精确表述：可见几何体在 OpenGL 里其实会"用满" [-1,1]
// （near → -1，far → +1），负半区并非空着浪费；[-1,1] 的问题是它对一个
// 本质非负的物理量采用了带符号的对称编码，并不携带额外信息。
//
// 3. [0,1] 的实际好处
//
// a) 与深度缓冲的存储格式天然对齐：深度缓冲硬件上存的是 unorm（无符号
//    归一化整数），天然就是 [0,1]——0 表示贴着近平面，1 表示顶到远平面。
//    OpenGL 流水线最后其实还要靠 glDepthRange 把 [-1,1] 折算回 [0,1]
//    才能存储，等于绕了个圈子；现代 API 让 clip space 直接就是 [0,1]，
//    省掉这层多余的约定转换。
//
// b) 配合反向深度（Reversed-Z）大幅提升精度：把近平面映射到 1、远平面
//    映射到 0，配合浮点深度缓冲，利用浮点数在 0 附近精度最高的特性，
//    可以大幅降低远处物体的 z-fighting。这套技巧在 [0,1] 区间下表达
//    得最干净。
//
// c) 裁剪更简单：0 <= z_c <= w_c 只比一个下界，硬件实现更直接。
//
// 有趣的是 OpenGL 后来也"认输"了：4.5 加入的 ARB_clip_control 扩展
// 允许把深度约定切成 [0,1]，就是为了方便移植 D3D 代码。这个转换矩阵
// 本质上就是在手工做一次 glClipControl 干的事。
//
// 补充：理解了这套原理之后，实践中也可以直接换用 WebGPU 友好的投影
// 写法（例如 glam 的 perspective_rh 直接输出 [0,1] 深度），就不再需要
// 这个转换矩阵了。
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::from_cols(
    cgmath::Vector4::new(1.0, 0.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 1.0, 0.0, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 0.0),
    cgmath::Vector4::new(0.0, 0.0, 0.5, 1.0),
);

struct Camera {
    /// 相机的位置，也就是"眼睛"所在的世界坐标
    eye: cgmath::Point3<f32>,
    /// 相机注视的目标点，相机的观察方向通常就是target - eye
    target: cgmath::Point3<f32>,
    /// 相机的"上方"方向，用来确定画面是否倾斜，常见值是(0, 1, 0)
    up: cgmath::Vector3<f32>,
    /// 视口宽高比，通常就是width / height，窗口大小改变的时候一般需要更新
    aspect: f32,
    /// 垂直方向的视野角（Field of View Y），角度越大，看到的范围越广，
    /// 这个后面具体要看单位是按弧度还是角度处理
    fovy: f32,
    /// 近裁剪面距离，比相机更近的物体不会被渲染，这个必须大于0
    znear: f32,
    /// 远裁剪面距离，比相机更远的物体不会被渲染
    afar: f32,
}

impl Camera {
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 对，你看到的这两个写法表面上方向完全相反，但实际上构造出来的是同一套 View
        // Matrix。关键在于：
        //
        //   cgmath 的 center - eye 表示的是摄像机实际"看过去"的方向；
        //   Song Ho 的 eye - center 表示的是摄像机坐标系的 +Z 轴（也就是摄像机的后方）。
        //
        // 而在 OpenGL 的右手坐标系 Eye Space 中，摄像机默认是朝 -Z 看的。Song Ho 的文章
        // 也明确说明，Eye Space 中相机位于原点并朝 -Z（见 https://www.songho.ca/opengl/gl_camera.html）。
        //
        // 所以这两个向量刚好应该互为相反数。
        //
        // 先看 cgmath，look_at_rh 的实现：
        //
        //   pub fn look_at_rh(
        //       eye: Point3<S>,
        //       center: Point3<S>,
        //       up: Vector3<S>,
        //   ) -> Matrix4<S> {
        //       Matrix4::look_to_rh(eye, center - eye, up)
        //   }
        //
        // 这里的 center - eye 非常直观，就是：
        //
        //   eye --------------------> center
        //            看过去的方向
        //
        // 假设 eye = (0, 0, 5)，center = (0, 0, 0)，那么：
        //
        //   center - eye
        //   = (0,0,0) - (0,0,5)
        //   = (0,0,-5)
        //
        // 也就是说摄像机朝世界空间的 -Z 看，这完全符合 OpenGL 默认相机朝 -Z 的约定。
        //
        // 但是重点来了：look_to_rh() 并没有直接把这个方向当成摄像机的 +Z 轴。
        // cgmath 的实现本质上是：
        //
        //   let f = dir.normalize();
        //   let s = f.cross(up).normalize();
        //   let u = s.cross(f);
        //
        // 然后构造矩阵的时候使用的是 s、u、-f。注意这个 -f：也就是说，虽然你传进去的
        // 是 f = center - eye，最终摄像机坐标系的 Z 基向量却是：
        //
        //   z_axis = -f
        //          = -(center - eye)
        //          = eye - center
        //
        // cgmath 源码确实如此：它计算 f = dir.normalize()，然后在矩阵中放入 -f。
        // 所以完整过程其实是：
        //
        //   dir = center - eye    // 摄像机实际朝向
        //   f   = normalize(dir)
        //
        //   camera +Z = -f
        //             = normalize(eye - center)
        //
        // 这就已经和 Song Ho 完全一样了。
        //
        // 再看 Song Ho 为什么直接用 eye - target。Song Ho 写的是：
        //
        //   Vector3 forward = eye - target;
        //   forward.normalize();
        //
        // 也就是：
        //
        //   center --------------------> eye
        //               forward
        //
        // 文章（https://www.songho.ca/opengl/gl_camera.html）中特别解释了这一点：
        // forward vector 是从 target 指向 eye，而不是 eye 指向 target。
        //
        // 这里最容易产生误解的是 forward 这个名字。这个 forward 并不是"摄像机正在看的
        // 方向"，而实际上是"摄像机局部坐标系的 +Z 方向"，也就是摄像机的"后方"。
        //
        // 因为 OpenGL Eye Space 是：
        //
        //                +Y
        //                 ↑
        //                 |
        //                 |
        //         -X ← camera → +X
        //                /
        //               /
        //             +Z
        //
        // 而摄像机真正看的方向是 -Z。所以如果我们定义 view_direction = center - eye，
        // 那么：
        //
        //   camera_Z = -view_direction
        //            = eye - center
        //
        // 也就是：
        //
        //               camera +Z
        //                   ↑
        //                   |
        //   center <--------- eye
        //          看向这里
        //
        //          <---------
        //          camera -Z
        //
        // 因此 center - eye 和 eye - center 其实分别是在描述两个不同概念：
        //
        //   center - eye : 摄像机真正的观察方向，即 local -Z
        //   eye - center : 摄像机坐标系的 local +Z 轴，也就是"backward"
        //
        // 用你的例子一眼就能看出来。假设 center = (0,0,0)，eye = (0,0,5)：
        //
        //              eye = (0,0,5)
        //                   ●
        //                   |
        //                   |
        //                   ↓ camera 看向这里
        //                   |
        //                   |
        //                   ●
        //              center = (0,0,0)
        //
        // 那么观察方向：
        //
        //   center - eye = (0,0,-5)
        //   normalize    = (0,0,-1)
        //
        // 所以 camera forward = -Z。而摄像机自己的 +Z：
        //
        //   eye - center = (0,0,+5)
        //   normalize    = (0,0,+1)
        //
        // 于是：
        //
        //                        camera +Z
        //                            ↑
        //                            |
        //                            ● eye
        //                            |
        //                            |
        //                            ↓ camera -Z
        //                            |
        //                            ● center
        //
        // 这正是 OpenGL 的标准相机约定。所以可以把两种实现理解成：
        //
        //   Song Ho: z = normalize(eye - center)
        //   cgmath : f = normalize(center - eye); z = -f
        //
        // 代入：
        //
        //   z = -normalize(center - eye)
        //     = normalize(eye - center)
        //
        // 完全相同。你之所以感觉冲突，主要是因为 Song Ho 把 eye - center 称为 forward
        // 有一点反直觉；如果按"摄像机真正朝哪里看"来命名，它其实更适合叫 backward 或者
        // camera_z_axis，这样就不容易混淆了。
        //
        // 参考：
        // - Songho《OpenGL Camera》: https://www.songho.ca/opengl/gl_camera.html
        // - cgmath 源码（look_at_rh -> look_to_rh，矩阵由 s、u、-f 构造）:
        //   https://android.googlesource.com/platform/external/rust/android-crates-io/+/ff914dc2eab1a11b38fbd51ee2f5a898efcbb0fc
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.afar);

        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    fn new() -> Self {
        use cgmath::SquareMatrix;

        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into()
    }
}

struct CameraController {
    speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
}

impl CameraController {
    fn new(speed: f32) -> Self {
        Self {
            speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
        }
    }

    fn handle_key(&mut self, code: KeyCode, is_pressed: bool) -> bool {
        match code {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.is_forward_pressed = is_pressed;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.is_left_pressed = is_pressed;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.is_backward_pressed = is_pressed;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.is_right_pressed = is_pressed;
                true
            }
            _ => false,
        }
    }

    fn update_camera(&self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Prevents glitching when the camera gets too close to the
        // center of the scene.
        if self.is_forward_pressed && forward_mag > self.speed {
            camera.eye += forward_norm * self.speed;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed;
        }

        let right = forward_norm.cross(camera.up);

        // Redo radius calc in case the forward/backward is pressed.
        let forward = camera.target - camera.eye;
        let forward_mag = forward.magnitude();

        if self.is_right_pressed {
            // Rescale the distance between the target and the eye so
            // that it doesn't change. The eye, therefore, still
            // lies on the circle made by the target and eye.
            camera.eye = camera.target - (forward + right * self.speed).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            camera.eye = camera.target - (forward - right * self.speed).normalize() * forward_mag;
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    const ATTRIBS: [wgpu::VertexAttribute; 2] =
        wgpu::vertex_attr_array![0 => Float32x3, 1=> Float32x2];

    fn desc() -> VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

const VERTICES: &[Vertex] = &[
    // Changed
    Vertex {
        position: [-0.0868241, 0.49240386, 0.0],
        tex_coords: [0.4131759, 0.00759614],
    }, // A
    Vertex {
        position: [-0.49513406, 0.06958647, 0.0],
        tex_coords: [0.0048659444, 0.43041354],
    }, // B
    Vertex {
        position: [-0.21918549, -0.44939706, 0.0],
        tex_coords: [0.28081453, 0.949397],
    }, // C
    Vertex {
        position: [0.35966998, -0.3473291, 0.0],
        tex_coords: [0.85967, 0.84732914],
    }, // D
    Vertex {
        position: [0.44147372, 0.2347359, 0.0],
        tex_coords: [0.9414737, 0.2652641],
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
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
    diffuse_bind_group: wgpu::BindGroup,
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

        // 这里的diffuse通常代表“漫反射纹理”，常见于图形学或者游戏材质系统
        let diffuse_bytes = include_bytes!("../media/image/happy-tree.png");

        // let diffuse_bytes = include_bytes!("happy-tree.png"); // CHANGED!
        // let diffuse_texture = texture::Texture::from_bytes(&device, &queue, diffuse_bytes, "happy-tree.png").unwrap(); // CHANGED!

        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        use image::GenericImageView;
        // 读取图片尺寸，返回 width 以及 height
        let dimensions = diffuse_image.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            // 所有Texture都作为3D来进行保存，通过将depth_or_array_layers设置为1
            // 来表示2D的Texture
            depth_or_array_layers: 1,
        };

        let diffuse_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("diffuse_texture"),
            // 纹理尺寸，类型通常是 wgpu::Extent3d, 其中width是宽度，height是高度
            // depth_or_array_layers是深度或者数组层数
            size: texture_size,
            // Mipmap层级数量，这里1表示只有原始尺寸的一层，不生成mipmap，如果使用mipmap
            // 则需要创建多层并准备相应数据
            mip_level_count: 1,
            // 每个纹理像素的采样数，1表示普通纹理，不使用多重采样MSAA，常规图片纹理一般使用1
            sample_count: 1,
            // 纹理维度，D2表示二维纹理，对应着色器之中的texture_2d，其它选项还有D1和D3
            dimension: wgpu::TextureDimension::D2,
            // 纹理之中每个像素的数据格式，Rgba8UnormSrgb表示每个像素有RGBA四个通道，然后每个通道8
            // 位；RGB按照sRGB颜色空间处理，Alpha通道通常按照线性值处理
            //
            // Unorm是Unsigned Normalized的缩写，中文通常叫做无符号归一化整数
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            // 纹理的用途，可以使用`|`组合成多个用途，这里包含两个用途：
            // 1. TEXTURE_BINDING表示可以创建纹理视图并绑定到对应的着色器
            // 2. COPY_DST表示可以通过queue.write_texture或者复制命令将数据写入纹理
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            // 允许纹理视图使用的额外格式，这和我们在创建SurfaceConfiguration的时候表示的内容指定的
            // 配置是相同的，它指定了可以使用哪些纹理格式来为此纹理创建TextureView，在这里纹理的基础
            // 格式（本例之中是Rgba8UnormSrgb）始终是支持的。
            //
            // 但是在WebGL2后端之中不支持使用不同的纹理格式
            view_formats: &[],
        });

        // Texture结构体没有直接操作数据的方法，但是可以使用之前创建的queue上面的write_texture
        // 方法来加载纹理
        queue.write_texture(
            // 告诉wgpu将纹理的rgba数据拷贝到这个地方
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            // texture的原始数据
            diffuse_rgba.as_raw(),
            // texture的具体布局格式
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        // 既然我们的纹理之中已经有了数据，那我们就需要一种使用它的方法。这就是TextireView和Sampler发挥
        // 作用的地方
        // TextireView为我们提供了观察纹理的视图。Sampler则控制Texture的采样方式
        // 采样的工作原理类似于吸管工具，我们的程序提供纹理上的一个坐标，然后采样器根据纹理和一些内部参数
        // 返回相应的颜色

        let diffuse_texture_view =
            diffuse_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let diffuse_sampler = device.create_sampler(&wgpu::wgt::SamplerDescriptor {
            // address_mode参数决定了当采样器获取到纹理范围之外的纹理坐标的时候应该如何处理
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            // mag_filter和min_filter表示的都是当纹理映射到屏幕的时候，应该如何从纹理之中获取颜色
            // 特别指当采样足迹小于或者大于一个纹素（texel）的时候应该如何处理
            //
            // 现在这里确定几个概念：
            // - 纹素（texel）:也就是纹理之中的一个像素
            // - 屏幕像素（pixel）:最终显示图像之中的一个像素
            // - 采样足迹（sample footprint）:也就是一个屏幕像素对应到纹理之中的区域大小
            //
            // 例如，一个 100×100 的纹理贴到屏幕上的 500×500 区域中，每个屏幕像素只对应纹
            // 理的一小部分，这叫放大；反过来，把它缩小到 20×20 区域，则一个屏幕像素可能对应多个纹素，
            // 这叫缩小
            //
            // 有两种选择：
            // - Linear: 在每个维度之中选择两个纹素，并返回其值之间的线性差值
            // - Nearest: 返回最靠近纹理坐标的纹素值。这会使图像在远处看起来更清晰，
            // 但在近处会呈现像素化。然而，如果你的纹理本身就是像素风格的设计，比如
            // 像素艺术游戏或像《我的世界》这样的体素游戏，这种效果可能正是你想要的
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        // 有了上面的这些资源当然很好了，但是如果我们没有办法将它们接入任何地方，它们就没多大用处
        // 这就是BindGroup以及PipelineLayout发挥作用的地方

        // BindGroup 描述了一组资源以及着色器如何访问这些资源。我们使用 BindGroupLayout 来创建一个 BindGroup

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("texture bind group layout"),
                // 这里包含两个条目，一个是位于绑定点0的采样纹理，一个是位于绑定点1的采样器
                // 这两者通常要配套使用：纹理提供"图像数据"，采样器决定"如何读取这些数据"
                //
                // 就像是FRAGMENT指定的，这两个绑定仅对片元着色器可见，虽然说这个值也可以指定
                // 成大部分其它的着色器阶段，但是大部分情况下就仅使用FRAGMENT.
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        // 这些个binding数字都必须和WGSL之中的@binding(0)对应
                        binding: 0,
                        // 表示这个资源允许被哪些着色器阶段访问
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // 表示这个绑定槽位存放的是纹理视图
                        ty: wgpu::BindingType::Texture {
                            // 表示从纹理之中读取出来的数据是浮点类型，以及允许使用过滤采样器，也就是
                            // 下面这个绑定项对应的Sampler
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            // 表示绑定的是2D纹理，通常用于：普通图片、精灵图、UI图片、2D游戏纹理、
                            // 以及材质贴图
                            view_dimension: wgpu::TextureViewDimension::D2,
                            // 表示这不是多重采样纹理，就是一个普通的单采样二维纹理
                            multisampled: false,
                        },
                        // 表示这个绑定槽就只绑定一个资源，而不是资源数组
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // 表示这里放的一个采样器而不是纹理，而且这里放的是一个支持过滤的采样器
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        // 也表示这里只绑定一个采样器
                        count: None,
                    },
                ],
            });

        // 有了BindGroupLayout，此时我们就可以开始创建BindGroup了
        //
        // BindGroup 是对 BindGroupLayout 更具体的声明。之所以将它们分开，是因为这允许我们在运行中
        // 快速切换 BindGroup ，只要它们都共享相同的 BindGroupLayout
        let diffuse_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("diffuse_bind_group"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                },
            ],
        });

        let shader = device.create_shader_module(wgpu::include_wgsl!("../media/shaders/hello-camera.wgsl"));

        // cgmath默认遵循OpenGL风格的右手坐标系：x向右、y向上、z朝向屏幕之外（指向观察者）
        // 所以项目要看到场景，就要站在+z方向，朝-z方向看
        let camera = Camera {
            eye: (0.0, 1.0, 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            afar: 100.0,
        };
        let camera_controller = CameraController::new(0.2);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera_bind_group_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera_bind_group"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline"),
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_bind_group_layout),
                ],
                immediate_size: 0,
            });

        // 可以把渲染管线想象成一个函数：
        // render_pipeline(
        //     bind_group_0,
        //     bind_group_1,
        //     ...,
        //     immediate_data
        // )
        //
        // bind_group_layouts: &[]：Shader 不接收任何绑定组资源，比如纹理、采样器、Uniform Buffer、Storage Buffer
        // immediate_size: 0：Shader 不接收 immediate data，也就是类似 Vulkan push constants 的少量快速参数
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(INDICES),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_indices = INDICES.len() as u32;

        Ok(Self {
            surface,
            device,
            queue,
            config,
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            is_surface_configured: false,
            window,
            // 现在我们已经有了一个相机，而且它能够生成view projection matrix，那现在
            // 我们就需要一个地方来放置它。同时我们也需要某种方法将其引入到着色器之中
            camera,
            camera_controller,
            camera_bind_group,
            camera_buffer,
            camera_uniform,
            diffuse_bind_group,
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

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, is_pressed: bool) {
        if code == KeyCode::Escape && is_pressed {
            event_loop.exit();
        } else {
            self.camera_controller.handle_key(code, is_pressed);
        }
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
            | wgpu::CurrentSurfaceTexture::Validation => return Ok(()),
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => anyhow::bail!("Lost device"),
        };

        let view = output
            .texture
            .create_view(&wgpu::wgt::TextureViewDescriptor::default());

        let mut encoder =
            self.device
                .create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor {
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
            render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        self.queue.present(output);

        Ok(())
    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
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
        event_loop: &winit::event_loop::ActiveEventLoop,
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
