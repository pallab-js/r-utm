<script lang="ts">
    interface Props {
        data: number[];
        color: string;
        label: string;
        max?: number;
        unit?: string;
        current?: number;
    }

    let { data, color, label, max = 100, unit = "%", current }: Props = $props();

    let canvas: HTMLCanvasElement;
    let width = $state(200);
    let height = $state(60);

    $effect(() => {
        if (canvas && data.length > 0) {
            const ctx = canvas.getContext("2d");
            if (!ctx) return;

            ctx.clearRect(0, 0, width, height);
            
            const padding = 2;
            const chartWidth = width - padding * 2;
            const chartHeight = height - padding * 2;

            ctx.strokeStyle = color;
            ctx.lineWidth = 1.5;
            ctx.beginPath();

            const step = chartWidth / Math.max(data.length - 1, 1);
            
            data.forEach((value, i) => {
                const x = padding + i * step;
                const normalizedValue = Math.min(value / max, 1);
                const y = padding + chartHeight - (normalizedValue * chartHeight);
                
                if (i === 0) {
                    ctx.moveTo(x, y);
                } else {
                    ctx.lineTo(x, y);
                }
            });

            ctx.stroke();

            ctx.fillStyle = color + "20";
            ctx.lineTo(padding + (data.length - 1) * step, height - padding);
            ctx.lineTo(padding, height - padding);
            ctx.closePath();
            ctx.fill();
        }
    });

    function formatValue(value: number): string {
        if (value >= 1000) {
            return `${(value / 1000).toFixed(1)}G`;
        }
        return value.toFixed(1);
    }
</script>

<div class="flex flex-col gap-1">
    <div class="flex items-center justify-between text-xs">
        <span class="text-neutral-400">{label}</span>
        <span class="text-white font-mono" style="color: {color}">
            {current !== undefined ? formatValue(current) : "--"}{unit}
        </span>
    </div>
    <canvas
        bind:this={canvas}
        bind:clientWidth={width}
        bind:clientHeight={height}
        class="w-full h-[60px] rounded bg-neutral-800/50"
    ></canvas>
</div>
