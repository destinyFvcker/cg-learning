<script lang="ts">
	import { onMount } from "svelte";

	async function main() {
		// 首先请求适配器，适配器代表一个特定的GPU，有些设备拥有多个GPU
		const adapter = await navigator.gpu.requestAdapter();
		// 从适配器请求设备
		const device = await adapter?.requestDevice();

		if (!device) {
			// 如果没有device可用，很可能是因为用户使用的是旧版的浏览器
			alert("need a browser that supports WebGPU");
			return;
		}

		// 然后找到canvas元素并为其创建一个webGPU上下文。这将使我们可以获取一个用于渲染的纹理，这个纹理将用于在网页之中显示canvas
		const canvas = document.querySelector("canvas");
		const context = canvas?.getContext("webgpu") as GPUCanvasContext | null | undefined;

		if (!context) {
			alert("could not get a WebGPU canvas context");
			return;
		}

		// 向系统查询首选的canvas格式。结果通常是"rgba8unorm"和"bgra8unorm"
		const presentationFormat = navigator.gpu.getPreferredCanvasFormat();
		// 传入device，将此画布和我们刚刚创建的设备关联起来
		context.configure({
			device,
			format: presentationFormat,
		});

		const module = device.createShaderModule({
			label: "our hardcoded red triangle shaders",
			code: /* wgsl */ `
                @vertex fn vs(
                  @builtin(vertex_index) vertexIndex : u32
                ) -> @builtin(position) vec4f {
                  let pos = array(
                    vec2f( 0.0,  0.5),  // top center
                    vec2f(-0.5, -0.5),  // bottom left
                    vec2f( 0.5, -0.5)   // bottom right
                  );
 
                  return vec4f(pos[vertexIndex], 0.0, 1.0);
                }
 
                @fragment fn fs() -> @location(0) vec4f {
                  return vec4f(1.0, 0.0, 0.0, 1.0);
                }
            `,
		});

		const pipeline = device.createRenderPipeline({
			label: "our hardcoded red triangle pipeline",
			layout: "auto",
			vertex: {
				entryPoint: "vs",
				module,
			},
			fragment: {
				entryPoint: "fs",
				module,
				targets: [{ format: presentationFormat }],
			},
		});

		function render(context: GPUCanvasContext, device: GPUDevice) {
			const renderPassDescriptor: GPURenderPassDescriptor = {
				label: "our basic canvas renderPass",
				colorAttachments: [
					{
						view: context.getCurrentTexture().createView(),
						clearValue: [0.3, 0.3, 0.3, 1],
						loadOp: "clear",
						storeOp: "store",
					},
				],
			};

			// make a command encoder to start encoding commands
			const encoder = device.createCommandEncoder({ label: "our encoder" });

			// make a render pass encoder to encode render specific commands
			const pass = encoder.beginRenderPass(renderPassDescriptor);
			pass.setPipeline(pipeline);
			pass.draw(3); // call our vertex shader 3 times.
			pass.end();

			const commandBuffer = encoder.finish();
			device.queue.submit([commandBuffer]);
		}

		render(context, device);
	}

	// 另外这是 SvelteKit 项目，访问 document 的代码最好放进 onMount()，
	// 避免服务端渲染时出现 document is not defined。
	onMount(() => {
		main();
	});
</script>

<canvas></canvas>
