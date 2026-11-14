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
        const w = width;
        const h = height;
        if (!canvas || w <= 0 || h <= 0) return;

        const ctx = canvas.getContext("2d");
        if (!ctx) return;

        const dpr = window.devicePixelRatio || 1;
        canvas.width = Math.round(w * dpr);
        canvas.height = Math.round(h * dpr);
        ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
        ctx.clearRect(0, 0, w, h);

        const styles = getComputedStyle(document.documentElement);
        const baseline = styles.getPropertyValue("--chart-baseline").trim() || "rgba(255,255,255,0.08)";
        const padding = 2;
        const chartWidth = w - padding * 2;
        const chartHeight = h - padding * 2;

        // baseline
        ctx.strokeStyle = baseline;
        ctx.lineWidth = 1;
        ctx.beginPath();
        ctx.moveTo(padding, h - padding);
        ctx.lineTo(w - padding, h - padding);
        ctx.stroke();

        if (data.length === 0) return;

        const step = chartWidth / Math.max(data.length - 1, 1);
        let points = data.map((value, i) => {
            const x = padding + i * step;
            const normalizedValue = Math.min(Math.max(value / max, 0), 1);
            const y = padding + chartHeight - normalizedValue * chartHeight;
            return { x, y };
        });

        // A single sample has no width to draw; render it as a flat line.
        if (points.length === 1) {
            points = [
                { x: padding, y: points[0].y },
                { x: w - padding, y: points[0].y },
            ];
        }

        // filled area
        ctx.beginPath();
        ctx.moveTo(points[0].x, h - padding);
        points.forEach((p) => ctx.lineTo(p.x, p.y));
        ctx.lineTo(points[points.length - 1].x, h - padding);
        ctx.closePath();
        ctx.fillStyle = color + "20";
        ctx.fill();

        // line
        ctx.beginPath();
        points.forEach((p, i) => (i === 0 ? ctx.moveTo(p.x, p.y) : ctx.lineTo(p.x, p.y)));
        ctx.strokeStyle = color;
        ctx.lineWidth = 1.5;
        ctx.lineJoin = "round";
        ctx.stroke();

        // emphasise the newest sample so a single reading is still legible
        const last = points[points.length - 1];
        ctx.beginPath();
        ctx.arc(last.x, last.y, 2, 0, Math.PI * 2);
        ctx.fillStyle = color;
        ctx.fill();
    });

    function formatValue(value: number): string {
        if (value >= 1000) {
            return `${(value / 1000).toFixed(1)}`;
        }
        return Number.isInteger(value) ? String(value) : value.toFixed(1);
    }
</script>

<div class="flex flex-col gap-1">
    <div class="flex items-center justify-between text-xs">
        <span class="text-[#898989]">{label}</span>
        <span class="text-white font-mono" style="color: {color}">
            {current !== undefined ? `${formatValue(current)} ${unit}` : "—"}
        </span>
    </div>
    <canvas
        bind:this={canvas}
        bind:clientWidth={width}
        bind:clientHeight={height}
        class="w-full h-[60px] rounded bg-[var(--color-bg-elevated)]"
    ></canvas>
</div>
