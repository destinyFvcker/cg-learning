<script lang="ts">
    import { onMount } from "svelte";
    import shaderStr from "$lib/shaders/data-pass/inter-stage/kimi-triangle.wgsl?raw";
    import { Button } from "$lib/components/ui/button/index.js";

    let canvas: HTMLCanvasElement;
    let mode: "canvas" | "uv" = $state("canvas");

    async function main(): Promise<(() => void) | undefined> {
        const adapter = await navigator.gpu.requestAdapter();
        const device = await adapter?.requestDevice();
        if (!device) {
            alert("need a browser that supports WebGPU");
            return;
        }

        const context = canvas.getContext("webgpu");
        if (!context) {
            alert("could not get a WebGPU canvas context");
            return;
        }

        const presentationFormat = navigator.gpu.getPreferredCanvasFormat();
        context.configure({ device, format: presentationFormat });

        const module = device.createShaderModule({ code: shaderStr });

        // 两个管线共享同一个 module 和顶点着色器，只是片段入口不同
        function makePipeline(fragmentEntry: string) {
            return device!.createRenderPipeline({
                layout: "auto",
                vertex: { module, entryPoint: "vs_main" },
                fragment: {
                    module,
                    entryPoint: fragmentEntry,
                    targets: [{ format: presentationFormat }],
                },
            });
        }
        const pipelines = {
            canvas: makePipeline("fs_canvas"),
            uv: makePipeline("fs_uv"),
        };

        // uniform 里放三角形的平移偏移量，让它左右滑动
        const uniformBuffer = device.createBuffer({
            size: 16, // uniform 结构体大小需按 16 字节对齐
            usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
        });

        // layout: "auto" 下每个管线的布局是独立对象，bindGroup 要分开建
        const bindGroups = {
            canvas: device.createBindGroup({
                layout: pipelines.canvas.getBindGroupLayout(0),
                entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
            }),
            uv: device.createBindGroup({
                layout: pipelines.uv.getBindGroupLayout(0),
                entries: [{ binding: 0, resource: { buffer: uniformBuffer } }],
            }),
        };

        // 关键 1：窗口变化时真正更新位图尺寸
        function resize() {
            const dpr = window.devicePixelRatio || 1;
            const rect = canvas.getBoundingClientRect();
            const width = Math.max(1, Math.round(rect.width * dpr));
            const height = Math.max(1, Math.round(rect.height * dpr));
            if (canvas.width !== width || canvas.height !== height) {
                canvas.width = width;
                canvas.height = height;
            }
        }
        const observer = new ResizeObserver(resize);
        observer.observe(canvas);
        resize();

        // 关键 2：每帧渲染并让三角形动起来
        let raf = 0;
        function frame(time: number) {
            const t = time / 1000;
            device!.queue.writeBuffer(
                uniformBuffer,
                0,
                new Float32Array([Math.sin(t) * 0.5, 0]),
            );

            const encoder = device!.createCommandEncoder();
            const pass = encoder.beginRenderPass({
                colorAttachments: [
                    {
                        view: context!.getCurrentTexture().createView(),
                        clearValue: [0.3, 0.3, 0.3, 1],
                        loadOp: "clear",
                        storeOp: "store",
                    },
                ],
            });
            pass.setPipeline(pipelines[mode]);
            pass.setBindGroup(0, bindGroups[mode]);
            pass.draw(3);
            pass.end();
            device!.queue.submit([encoder.finish()]);

            raf = requestAnimationFrame(frame);
        }
        raf = requestAnimationFrame(frame);

        return () => {
            cancelAnimationFrame(raf);
            observer.disconnect();
            uniformBuffer.destroy();
            device!.destroy();
        };
    }

    onMount(() => {
        let cleanup: (() => void) | undefined;
        main().then((c) => (cleanup = c));
        return () => cleanup?.();
    });
</script>

<div class="flex flex-col gap-3">
    <div class="flex gap-2 text-sm">
        <!-- svelte-ignore event_directive_deprecated -->
        <Button
            class="rounded border px-3 py-1"
            onclick={() => (mode = "canvas")}
            variant={mode == "uv" ? "outline" : "default"}
        >
            画布像素坐标（有问题的版本）
        </Button>
        <!-- svelte-ignore event_directive_deprecated -->
        <Button
            class="rounded border px-3 py-1"
            onclick={() => (mode = "uv")}
            variant={mode == "uv" ? "default" : "outline"}
        >
            <!-- class:text-white={mode === "uv"} -->
            三角形 UV（修正版）
        </Button>
    </div>

    <!-- 给 canvas 一个明确的 CSS 高度，否则它默认只有 150px -->
    <canvas bind:this={canvas} class="block h-72 w-full border"></canvas>

    <p class="text-sm text-gray-500">
        观察三角形左右滑动时的棋盘格，再拖动浏览器窗口改变画布宽度试试。
    </p>
</div>
