import { invoke } from "@tauri-apps/api/core";
import { onDestroy } from "svelte";

export interface VmMetrics {
    vm_id: string;
    cpu_percent: number;
    memory_mb: number;
    uptime_seconds: number;
}

export interface MetricsHistory {
    cpu: number[];
    memory: number[];
    timestamps: number[];
}

const MAX_HISTORY = 60;

export function createMetricsStore(vmId: string) {
    let metrics = $state<VmMetrics | null>(null);
    let history = $state<MetricsHistory>({
        cpu: [],
        memory: [],
        timestamps: [],
    });
    let isPolling = $state(false);
    let pollInterval: ReturnType<typeof setInterval> | null = null;

    async function fetchMetrics() {
        try {
            const result = await invoke<VmMetrics | null>("get_vm_metrics", { vmId });
            if (result) {
                metrics = result;
                history.cpu = [...history.cpu, result.cpu_percent].slice(-MAX_HISTORY);
                history.memory = [...history.memory, result.memory_mb].slice(-MAX_HISTORY);
                history.timestamps = [...history.timestamps, Date.now()].slice(-MAX_HISTORY);
            }
        } catch (e) {
            console.error("Failed to fetch metrics:", e);
        }
    }

    function startPolling(intervalMs = 1000) {
        if (isPolling) return;
        isPolling = true;
        fetchMetrics();
        pollInterval = setInterval(fetchMetrics, intervalMs);
    }

    function stopPolling() {
        if (pollInterval) {
            clearInterval(pollInterval);
            pollInterval = null;
        }
        isPolling = false;
    }

    function reset() {
        stopPolling();
        metrics = null;
        history = { cpu: [], memory: [], timestamps: [] };
    }

    onDestroy(() => {
        stopPolling();
    });

    return {
        get metrics() { return metrics; },
        get history() { return history; },
        get isPolling() { return isPolling; },
        startPolling,
        stopPolling,
        reset,
    };
}
