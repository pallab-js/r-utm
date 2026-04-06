<script lang="ts">
    import { onMount, onDestroy } from "svelte";
    import { invoke } from "@tauri-apps/api/core";

    interface Props {
        vmId: string;
        spicePort: number;
        onClose?: () => void;
    }

    let { vmId, spicePort, onClose }: Props = $props();

    let canvas: HTMLCanvasElement;
    let container: HTMLDivElement;
    let ctx: CanvasRenderingContext2D | null = null;
    let width = $state(800);
    let height = $state(600);
    let connected = $state(false);
    let connecting = $state(true);
    let error = $state<string | null>(null);
    let ws: WebSocket | null = null;
    let imageData: ImageData | null = null;

    const SPICE_MSGTYPE_DISPLAY_INIT = 101;
    const SPICE_MSGTYPE_DISPLAY_MODE = 102;
    const SPICE_MSGTYPE_DISPLAY_MARK = 103;
    const SPICE_MSGTYPE_DISPLAY_RESET = 104;
    const SPICE_MSGTYPE_DISPLAY_COPY_BITS = 105;
    const SPICE_MSGTYPE_DISPLAY_INVAL_ALL = 106;
    const SPICE_MSGTYPE_DISPLAY_INVAL_ONE = 107;
    const SPICE_MSGTYPE_DISPLAY_UPDATE = 108;
    const SPICE_MSGTYPE_DISPLAY_SURFACE_CREATE = 109;
    const SPICE_MSGTYPE_DISPLAY_SURFACE_DESTROY = 110;
    const SPICE_MSGTYPE_DISPLAY_STREAM_CREATE = 111;
    const SPICE_MSGTYPE_DISPLAY_STREAM_DATA = 112;
    const SPICE_MSGTYPE_DISPLAY_STREAM_CLIP = 113;
    const SPICE_MSGTYPE_DISPLAY_STREAM_DESTROY = 114;
    const SPICE_MSGTYPE_DISPLAY_STREAM_END = 115;
    const SPICE_MSGTYPE_DISPLAY_STREAM_SEND = 116;

    const SPICE_CHANNEL_DISPLAY = 2;

    onMount(() => {
        setupCanvas();
        connectSpice();
        window.addEventListener("resize", handleResize);
    });

    onDestroy(() => {
        disconnect();
        window.removeEventListener("resize", handleResize);
    });

    function handleResize() {
        if (container) {
            width = container.clientWidth;
            height = container.clientHeight;
            if (canvas) {
                canvas.width = width;
                canvas.height = height;
                setupCanvas();
            }
        }
    }

    function setupCanvas() {
        if (!canvas) return;
        
        ctx = canvas.getContext("2d");
        if (!ctx) return;

        canvas.width = width;
        canvas.height = height;

        imageData = ctx.createImageData(width, height);
        
        ctx.fillStyle = "#1a1a2e";
        ctx.fillRect(0, 0, width, height);
        
        ctx.fillStyle = "#ffffff";
        ctx.font = "16px system-ui, -apple-system, sans-serif";
        ctx.textAlign = "center";
        ctx.fillText("SPICE Display - Connecting...", width / 2, height / 2);
    }

    async function connectSpice() {
        connecting = true;
        error = null;
        
        try {
            const wsUrl = `ws://127.0.0.1:${spicePort}`;
            ws = new WebSocket(wsUrl);
            ws.binaryType = "arraybuffer";
            
            ws.onopen = () => {
                connected = true;
                connecting = false;
                console.log("SPICE WebSocket connected");
                drawConnectingScreen("Connected, waiting for display data...");
            };

            ws.onclose = () => {
                connected = false;
                connecting = false;
                console.log("SPICE WebSocket disconnected");
                drawConnectingScreen("Disconnected from SPICE server");
            };

            ws.onerror = (e) => {
                error = "Connection error. Make sure QEMU is running with SPICE enabled.";
                connecting = false;
                connected = false;
                console.error("SPICE error:", e);
                drawConnectingScreen(error || "Connection failed");
            };

            ws.onmessage = (event) => {
                handleSpiceMessage(event.data);
            };
        } catch (e) {
            error = `Failed to connect: ${e}`;
            connecting = false;
            console.error("Failed to connect to SPICE:", e);
            drawConnectingScreen(error || "Connection failed");
        }
    }

    function drawConnectingScreen(message: string) {
        if (!ctx) return;
        
        ctx.fillStyle = "#1a1a2e";
        ctx.fillRect(0, 0, width, height);
        
        ctx.fillStyle = "#6366f1";
        ctx.font = "bold 18px system-ui, -apple-system, sans-serif";
        ctx.textAlign = "center";
        ctx.fillText("VibeVisor SPICE Display", width / 2, height / 2 - 30);
        
        ctx.fillStyle = "#a1a1aa";
        ctx.font = "14px system-ui, -apple-system, sans-serif";
        ctx.fillText(message, width / 2, height / 2);
        
        ctx.fillStyle = "#52525b";
        ctx.font = "12px system-ui, -apple-system, sans-serif";
        ctx.fillText(`Port: ${spicePort} | VM: ${vmId}`, width / 2, height / 2 + 30);
    }

    function disconnect() {
        if (ws) {
            ws.close();
            ws = null;
        }
    }

    function handleSpiceMessage(data: ArrayBuffer | Blob | string) {
        if (typeof data === "string") {
            try {
                const parsed = JSON.parse(data);
                if (parsed.type === "spice-init") {
                    handleSpiceInit(parsed);
                } else if (parsed.type === "display-mode") {
                    handleDisplayMode(parsed);
                } else if (parsed.type === "display-update") {
                    handleDisplayUpdate(parsed);
                } else if (parsed.type === "error") {
                    error = parsed.message;
                    console.error("SPICE error:", parsed.message);
                }
            } catch (e) {
                console.error("Failed to parse SPICE message:", e);
            }
            return;
        }

        if (data instanceof Blob) {
            const reader = new FileReader();
            reader.onload = () => {
                if (reader.result instanceof ArrayBuffer) {
                    processSpiceBytes(new Uint8Array(reader.result));
                }
            };
            reader.readAsArrayBuffer(data);
            return;
        }

        processSpiceBytes(new Uint8Array(data));
    }

    function processSpiceBytes(bytes: Uint8Array) {
        if (bytes.length < 4) return;

        const type = bytes[0] | (bytes[1] << 8);
        
        switch (type) {
            case SPICE_MSGTYPE_DISPLAY_INIT:
                handleDisplayInit(bytes);
                break;
            case SPICE_MSGTYPE_DISPLAY_MODE:
                handleDisplayModeMessage(bytes);
                break;
            case SPICE_MSGTYPE_DISPLAY_UPDATE:
                handleDisplayUpdateMessage(bytes);
                break;
            case SPICE_MSGTYPE_DISPLAY_COPY_BITS:
                handleCopyBits();
                break;
            default:
                if (bytes.length > 20) {
                    const updateData = extractImageData(bytes.subarray(20));
                    if (updateData && ctx) {
                        ctx.putImageData(updateData, 0, 0);
                    }
                }
                break;
        }
    }

    function handleSpiceInit(data: any) {
        console.log("SPICE initialized:", data);
        drawConnectingScreen("SPICE initialized, receiving display data...");
    }

    function handleDisplayMode(data: any) {
        if (data.width && data.height) {
            width = data.width;
            height = data.height;
            if (canvas) {
                canvas.width = width;
                canvas.height = height;
            }
            setupCanvas();
        }
    }

    function handleDisplayUpdate(data: any) {
        if (data.imageData && ctx) {
            const imgData = new ImageData(
                new Uint8ClampedArray(data.imageData),
                width,
                height
            );
            ctx.putImageData(imgData, data.x || 0, data.y || 0);
        }
    }

    function handleDisplayInit(bytes: Uint8Array) {
        console.log("Display init received");
    }

    function handleDisplayModeMessage(bytes: Uint8Array) {
        if (bytes.length >= 20) {
            const newWidth = bytes[4] | (bytes[5] << 8);
            const newHeight = bytes[8] | (bytes[9] << 8);
            
            if (newWidth > 0 && newHeight > 0 && newWidth < 8192 && newHeight < 8192) {
                width = newWidth;
                height = newHeight;
                if (canvas) {
                    canvas.width = width;
                    canvas.height = height;
                }
                setupCanvas();
                console.log(`Display mode changed: ${width}x${height}`);
            }
        }
    }

    function handleDisplayUpdateMessage(bytes: Uint8Array) {
        if (bytes.length < 24 || !ctx || !imageData) return;

        const x = bytes[4] | (bytes[5] << 8);
        const y = bytes[8] | (bytes[9] << 8);
        const w = bytes[12] | (bytes[13] << 8);
        const h = bytes[16] | (bytes[17] << 8);

        if (w > 0 && h > 0 && x + w <= width && y + h <= height) {
            const data = extractImageData(bytes.subarray(24));
            if (data) {
                ctx.putImageData(data, x, y);
            }
        }
    }

    function extractImageData(pixelData: Uint8Array): ImageData | null {
        if (!ctx || pixelData.length < 4) return null;
        
        const imgData = ctx.createImageData(width, height);
        const data = imgData.data;
        
        for (let i = 0; i < pixelData.length && i < data.length; i += 4) {
            data[i] = pixelData[i];     
            data[i + 1] = pixelData[i + 1]; 
            data[i + 2] = pixelData[i + 2]; 
            data[i + 3] = 255; 
        }
        
        return imgData;
    }

    function handleCopyBits() {
        console.log("Copy bits received");
    }

    function handleKeydown(e: KeyboardEvent) {
        if (!ws || ws.readyState !== WebSocket.OPEN) return;

        const keyMap: Record<string, number> = {
            "Enter": 0x1c,
            "Escape": 0x01,
            "Backspace": 0x0e,
            "Tab": 0x0f,
            "ArrowUp": 0x48,
            "ArrowDown": 0x50,
            "ArrowLeft": 0x4b,
            "ArrowRight": 0x4d,
            "Space": 0x39,
            "Delete": 0x53,
        };

        const scancode = keyMap[e.key] || e.key.charCodeAt(0);
        
        const msg = {
            type: "key",
            scancode,
            pressed: true,
            vmId,
        };
        
        ws.send(JSON.stringify(msg));

        setTimeout(() => {
            if (ws && ws.readyState === WebSocket.OPEN) {
                ws.send(JSON.stringify({
                    type: "key",
                    scancode,
                    pressed: false,
                    vmId,
                }));
            }
        }, 100);
    }

    function handleKeyup(e: KeyboardEvent) {
        if (!ws || ws.readyState !== WebSocket.OPEN) return;

        const keyMap: Record<string, number> = {
            "Enter": 0x1c,
            "Escape": 0x01,
            "Backspace": 0x0e,
            "Tab": 0x0f,
        };

        const scancode = keyMap[e.key] || e.key.charCodeAt(0);
        
        const msg = {
            type: "key",
            scancode,
            pressed: false,
            vmId,
        };
        
        ws.send(JSON.stringify(msg));
    }

    let lastMouseX = 0;
    let lastMouseY = 0;

    function handleMouseMove(e: MouseEvent) {
        if (!canvas || !ws || ws.readyState !== WebSocket.OPEN) return;

        const rect = canvas.getBoundingClientRect();
        const x = Math.floor((e.clientX - rect.left) * (canvas.width / rect.width));
        const y = Math.floor((e.clientY - rect.top) * (canvas.height / rect.height));

        if (x !== lastMouseX || y !== lastMouseY) {
            lastMouseX = x;
            lastMouseY = y;
            
            const msg = {
                type: "mouse",
                x,
                y,
                buttons: e.buttons,
                vmId,
            };
            ws.send(JSON.stringify(msg));
        }
    }

    function handleMouseDown(e: MouseEvent) {
        if (!ws || ws.readyState !== WebSocket.OPEN) return;

        const buttonMap: Record<number, number> = {
            0: 1,
            1: 2,
            2: 4,
        };

        const msg = {
            type: "mouse",
            x: lastMouseX,
            y: lastMouseY,
            buttons: buttonMap[e.button] || 0,
            buttonDown: true,
            vmId,
        };
        ws.send(JSON.stringify(msg));
    }

    function handleMouseUp(e: MouseEvent) {
        if (!ws || ws.readyState !== WebSocket.OPEN) return;

        const buttonMap: Record<number, number> = {
            0: 1,
            1: 2,
            2: 4,
        };

        const msg = {
            type: "mouse",
            x: lastMouseX,
            y: lastMouseY,
            buttons: 0,
            buttonDown: false,
            vmId,
        };
        ws.send(JSON.stringify(msg));
    }
</script>

<div 
    bind:this={container}
    class="fixed inset-0 z-50 bg-black flex flex-col"
>
    <div class="h-10 bg-neutral-900 border-b border-neutral-800 flex items-center justify-between px-4 shrink-0">
        <div class="flex items-center gap-3">
            <span class="w-2 h-2 rounded-full {connected ? 'bg-emerald-500 animate-pulse' : 'bg-red-500'}"></span>
            <span class="text-sm text-neutral-300">SPICE Display - {vmId}</span>
            {#if connecting}
                <span class="text-xs text-neutral-500">Connecting...</span>
            {/if}
        </div>
        <div class="flex items-center gap-2">
            <span class="text-xs text-neutral-500">{width}x{height}</span>
            <button
                onclick={onClose}
                aria-label="Close"
                class="p-1 rounded hover:bg-neutral-800 text-neutral-400 transition-colors"
            >
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
                </svg>
            </button>
        </div>
    </div>
    
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div 
        bind:this={container}
        class="flex-1 relative"
        tabindex="0"
        onkeydown={handleKeydown}
        onkeyup={handleKeyup}
        onmousemove={handleMouseMove}
        onmousedown={handleMouseDown}
        onmouseup={handleMouseUp}
        role="application"
    >
        <canvas
            bind:this={canvas}
            {width}
            {height}
            class="w-full h-full"
            style="image-rendering: pixelated; cursor: crosshair;"
        ></canvas>
        
        {#if !connected && !connecting}
            <div class="absolute inset-0 flex items-center justify-center bg-black/80">
                <div class="text-center max-w-md">
                    <div class="w-16 h-16 mx-auto mb-4 rounded-full bg-neutral-800 flex items-center justify-center">
                        <svg class="w-8 h-8 text-neutral-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
                        </svg>
                    </div>
                    <h3 class="text-lg font-medium text-white mb-2">SPICE Display</h3>
                    <p class="text-sm text-neutral-400 mb-4">
                        {#if error}
                            {error}
                        {:else}
                            Not connected to VM display
                        {/if}
                    </p>
                    <p class="text-xs text-neutral-600">
                        Make sure the VM is running with SPICE enabled
                    </p>
                </div>
            </div>
        {/if}
    </div>
    
    <div class="h-8 bg-neutral-900 border-t border-neutral-800 flex items-center justify-between px-4 text-xs text-neutral-500 shrink-0">
        <span>Click to capture mouse | Esc to release | Keyboard input enabled</span>
        <span>Port: {spicePort}</span>
    </div>
</div>
