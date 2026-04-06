<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { save, open, message } from "@tauri-apps/plugin-dialog";
  import Chart from "$lib/components/Chart.svelte";
  import Settings from "$lib/components/Settings.svelte";
  import CreateVmWizard from "$lib/components/CreateVmWizard.svelte";
  import EditVmModal from "$lib/components/EditVmModal.svelte";
  import SpiceDisplay from "$lib/components/SpiceDisplay.svelte";

  interface VmConfig {
    name: string;
    id: string;
    architecture: string;
    hardware: {
      cpu_cores: number;
      memory_mb: number;
    };
    storage: {
      disk_size_gb: number;
    };
    network: {
      enabled: boolean;
      mode: string;
    };
    display: {
      resolution: {
        width: number;
        height: number;
      };
    };
  }

  interface VmInfo {
    config: VmConfig;
    state: string;
    pid: number | null;
  }

  interface VmMetrics {
    vm_id: string;
    cpu_percent: number;
    memory_mb: number;
    uptime_seconds: number;
  }

  interface Toast {
    id: number;
    message: string;
    type: "success" | "error" | "info";
  }

  let vms = $state<VmInfo[]>([]);
  let showCreateModal = $state(false);
  let showSettings = $state(false);
  let showEditModal = $state(false);
  let editingVm = $state<VmInfo | null>(null);
  let showSpiceDisplay = $state(false);
  let spiceVmId = $state<string | null>(null);
  let spicePort = $state(5900);
  let selectedVm = $state<string | null>(null);
  let vmMetrics = $state<Record<string, VmMetrics>>({});
  let metricsInterval: ReturnType<typeof setInterval> | null = null;
  let toasts = $state<Toast[]>([]);
  let toastId = 0;

  let cmdPaletteOpen = $state(false);
  let cmdQuery = $state("");
  let closeConfirmationUnlisten: (() => void) | null = null;

  onMount(async () => {
    await loadVms();
    startMetricsPolling();
    
    // Listen for close confirmation events from backend
    closeConfirmationUnlisten = await listen<string>("close-confirmation-needed", async (event) => {
      const runningCount = parseInt(event.payload);
      
      const shouldQuit = await message(
        `${runningCount} VM(s) are currently running. Closing VibeVisor will stop all running VMs. Do you want to continue?`,
        {
          title: "Confirm Close",
          kind: "warning"
        }
      );
      
      if (shouldQuit) {
        await invoke("confirm_and_quit");
      }
    });
  });

  onDestroy(() => {
    if (closeConfirmationUnlisten) {
      closeConfirmationUnlisten();
    }
    if (metricsInterval) {
      clearInterval(metricsInterval);
    }
  });

  function showToast(message: string, type: Toast["type"] = "info") {
    const id = ++toastId;
    toasts = [...toasts, { id, message, type }];
    setTimeout(() => {
      toasts = toasts.filter(t => t.id !== id);
    }, 4000);
  }

  function startMetricsPolling() {
    metricsInterval = setInterval(async () => {
      if (selectedVm && vms.find(v => v.config.id === selectedVm)?.state === "Running") {
        try {
          const metrics = await invoke<VmMetrics | null>("get_vm_metrics", { vmId: selectedVm });
          if (metrics) {
            vmMetrics = { ...vmMetrics, [metrics.vm_id]: metrics };
          }
        } catch (e) {
          console.error("Failed to fetch metrics:", e);
        }
      }
    }, 1000);
  }

  async function loadVms() {
    try {
      vms = await invoke<VmInfo[]>("list_vms");
    } catch (e) {
      showToast(`Failed to load VMs: ${e}`, "error");
    }
  }

  async function handleVmCreate(event: CustomEvent<{ name: string; cpu: number; memory: number; disk: number; iso: string | null }>) {
    const { name, cpu, memory, disk, iso } = event.detail;
    try {
      await invoke("create_vm", {
        name,
        cpuCores: cpu,
        memoryMb: memory,
        diskSizeGb: disk,
        isoPath: iso,
      });
      showCreateModal = false;
      showToast(`Created VM: ${name}`, "success");
      await loadVms();
    } catch (e) {
      showToast(`Failed to create VM: ${e}`, "error");
    }
  }

  async function createVm(name: string, cpu: number, memory: number, disk: number, isoPath: string | null) {
    try {
      await invoke("create_vm", {
        name,
        cpuCores: cpu,
        memoryMb: memory,
        diskSizeGb: disk,
        isoPath,
      });
      showCreateModal = false;
      showToast(`Created VM: ${name}`, "success");
      await loadVms();
    } catch (e) {
      showToast(`Failed to create VM: ${e}`, "error");
    }
  }

  async function startVm(vmId: string) {
    try {
      await invoke("start_vm", { vmId });
      showToast("VM started", "success");
      await loadVms();
    } catch (e) {
      showToast(`Failed to start VM: ${e}`, "error");
    }
  }

  async function stopVm(vmId: string) {
    try {
      await invoke("stop_vm", { vmId });
      showToast("VM stopped", "info");
      await loadVms();
      delete vmMetrics[vmId];
    } catch (e) {
      showToast(`Failed to stop VM: ${e}`, "error");
    }
  }

  async function pauseVm(vmId: string) {
    try {
      await invoke("pause_vm", { vmId });
      showToast("VM paused", "info");
      await loadVms();
    } catch (e) {
      showToast(`Failed to pause VM: ${e}`, "error");
    }
  }

  async function resumeVm(vmId: string) {
    try {
      await invoke("resume_vm", { vmId });
      showToast("VM resumed", "success");
      await loadVms();
    } catch (e) {
      showToast(`Failed to resume VM: ${e}`, "error");
    }
  }

  async function deleteVm(vmId: string) {
    try {
      await invoke("delete_vm", { vmId });
      if (selectedVm === vmId) selectedVm = null;
      delete vmMetrics[vmId];
      showToast("VM deleted", "info");
      await loadVms();
    } catch (e) {
      showToast(`Failed to delete VM: ${e}`, "error");
    }
  }

  function editVm(vmId: string) {
    const vm = vms.find(v => v.config.id === vmId);
    if (vm) {
      editingVm = vm;
      showEditModal = true;
    }
  }

  function handleVmUpdated(updatedVm: VmConfig) {
    const index = vms.findIndex(v => v.config.id === updatedVm.id);
    if (index !== -1) {
      vms[index].config = updatedVm;
    }
  }

  async function openSpiceDisplay(vmId: string) {
    try {
      const port = await invoke<number>("get_spice_port", { vmId });
      spicePort = port;
      spiceVmId = vmId;
      showSpiceDisplay = true;
    } catch (e) {
      showToast(`Failed to get SPICE port: ${e}`, "error");
    }
  }

  async function exportVm(vmId: string) {
    try {
      const vm = vms.find(v => v.config.id === vmId);
      if (!vm) return;

      const filePath = await save({
        defaultPath: `${vm.config.name.replace(/[^a-zA-Z0-9]/g, "_")}.tar.gz`,
        filters: [{ name: "Archive", extensions: ["tar.gz"] }]
      });

      if (filePath) {
        const dir = filePath.substring(0, filePath.lastIndexOf("/"));
        await invoke("export_vm", { vmId, exportPath: dir });
        showToast(`Exported ${vm.config.name} successfully`, "success");
      }
    } catch (e) {
      showToast(`Failed to export VM: ${e}`, "error");
    }
  }

  async function importVm() {
    try {
      const filePath = await open({
        filters: [{ name: "Archive", extensions: ["tar.gz"] }]
      });

      if (filePath) {
        const newVm = await invoke<VmConfig>("import_vm", { archivePath: filePath });
        showToast(`Imported ${newVm.name} successfully`, "success");
        await loadVms();
      }
    } catch (e) {
      showToast(`Failed to import VM: ${e}`, "error");
    }
  }

  function selectVm(vmId: string) {
    selectedVm = selectedVm === vmId ? null : vmId;
  }

  function handleKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      cmdPaletteOpen = !cmdPaletteOpen;
    }
    if (e.key === "Escape") {
      cmdPaletteOpen = false;
    }
  }

  function formatMemory(mb: number): string {
    if (mb >= 1024) return `${mb / 1024} GB`;
    return `${mb} MB`;
  }

  function formatUptime(seconds: number): string {
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = seconds % 60;
    if (h > 0) return `${h}h ${m}m`;
    if (m > 0) return `${m}m ${s}s`;
    return `${s}s`;
  }

  function getStateColor(state: string): string {
    switch (state) {
      case "Running": return "bg-emerald-500";
      case "Paused": return "bg-yellow-500";
      case "Stopped": return "bg-neutral-500";
      default: return "bg-neutral-500";
    }
  }

  function getSelectedVm(): VmInfo | undefined {
    return vms.find(v => v.config.id === selectedVm);
  }

  let cpuHistory = $state<number[]>([]);
  let memHistory = $state<number[]>([]);

  $effect(() => {
    if (selectedVm && vmMetrics[selectedVm]) {
      const m = vmMetrics[selectedVm];
      cpuHistory = [...cpuHistory, m.cpu_percent].slice(-60);
      memHistory = [...memHistory, m.memory_mb].slice(-60);
    }
  });
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-screen w-screen bg-neutral-950 text-neutral-100 flex overflow-hidden font-sans">
  <aside class="w-64 h-full backdrop-blur-xl bg-neutral-900/50 border-r border-neutral-800 flex flex-col">
    <div class="p-6 border-b border-neutral-800">
      <h1 class="text-xl font-semibold tracking-tight text-white">VibeVisor</h1>
      <p class="text-xs text-neutral-500 mt-1">Apple Silicon VM Manager</p>
    </div>
    
    <nav class="flex-1 p-4 space-y-1">
      <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg bg-neutral-800 text-white text-sm">
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
        </svg>
        Virtual Machines
      </button>
      <button
        onclick={() => (showSettings = true)}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-lg text-neutral-400 hover:bg-neutral-800/50 hover:text-white text-sm transition-colors"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
        </svg>
        Settings
      </button>
    </nav>
    
    <div class="p-4 border-t border-neutral-800">
      <button
        onclick={() => (showCreateModal = true)}
        class="w-full flex items-center justify-center gap-2 px-4 py-2.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium transition-colors"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
        </svg>
        New VM
      </button>
    </div>
  </aside>

  <main class="flex-1 flex flex-col overflow-hidden">
    <header class="h-14 px-6 flex items-center justify-between border-b border-neutral-800 bg-neutral-900/30">
      <h2 class="text-sm font-medium text-neutral-300">Virtual Machines</h2>
      <div class="flex items-center gap-3">
        <button
          onclick={() => importVm()}
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-400 text-xs transition-colors"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
          Import
        </button>
        <button
          onclick={() => (cmdPaletteOpen = true)}
          class="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-neutral-800 hover:bg-neutral-700 text-neutral-400 text-xs transition-colors"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <span>Search...</span>
          <kbd class="ml-2 px-1.5 py-0.5 rounded bg-neutral-700 text-neutral-500">⌘K</kbd>
        </button>
      </div>
    </header>

    <div class="flex-1 flex overflow-hidden">
      <div class="flex-1 overflow-auto p-6">
        {#if vms.length === 0}
          <div class="h-full flex flex-col items-center justify-center text-center">
            <div class="w-16 h-16 rounded-2xl bg-neutral-800/50 flex items-center justify-center mb-4">
              <svg class="w-8 h-8 text-neutral-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 12h14M12 5v14" />
              </svg>
            </div>
            <h3 class="text-lg font-medium text-neutral-300 mb-2">No Virtual Machines</h3>
            <p class="text-sm text-neutral-500 mb-6 max-w-sm">
              Create your first virtual machine to get started with VibeVisor.
            </p>
            <button
              onclick={() => (showCreateModal = true)}
              class="flex items-center gap-2 px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-sm font-medium transition-colors"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
              </svg>
              Create VM
            </button>
          </div>
        {:else}
          <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
            {#each vms as vm}
              <div
                role="button"
                tabindex="0"
                onclick={() => selectVm(vm.config.id)}
                onkeydown={(e) => e.key === 'Enter' && selectVm(vm.config.id)}
                class="group relative text-left p-4 rounded-xl bg-neutral-900 border transition-all duration-200 cursor-pointer {selectedVm === vm.config.id ? 'border-blue-500 ring-1 ring-blue-500/50' : 'border-neutral-800 hover:border-neutral-700'}"
              >
                <div class="flex items-start justify-between mb-3">
                  <div class="flex items-center gap-3">
                    <div class="w-10 h-10 rounded-lg bg-neutral-800 flex items-center justify-center">
                      <svg class="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
                      </svg>
                    </div>
                    <div>
                      <h3 class="font-medium text-white">{vm.config.name}</h3>
                      <p class="text-xs text-neutral-500">{vm.config.architecture.toUpperCase()}</p>
                    </div>
                  </div>
                  <div class="flex items-center gap-1.5">
                    <span class="w-2 h-2 rounded-full {getStateColor(vm.state)}"></span>
                    <span class="text-xs text-neutral-500">{vm.state}</span>
                  </div>
                </div>
                
                <div class="space-y-2 text-sm">
                  <div class="flex justify-between text-neutral-400">
                    <span>CPU</span>
                    <span>{vm.config.hardware.cpu_cores} cores</span>
                  </div>
                  <div class="flex justify-between text-neutral-400">
                    <span>Memory</span>
                    <span>{formatMemory(vm.config.hardware.memory_mb)}</span>
                  </div>
                  <div class="flex justify-between text-neutral-400">
                    <span>Disk</span>
                    <span>{vm.config.storage.disk_size_gb} GB</span>
                  </div>
                </div>
                
                <div class="mt-4 pt-4 border-t border-neutral-800 flex gap-2">
                  {#if vm.state === "Running"}
                    <button
                      onclick={(e) => { e.stopPropagation(); openSpiceDisplay(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-blue-600/20 text-blue-400 hover:bg-blue-600/30 text-xs font-medium transition-colors"
                    >
                      Display
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); pauseVm(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-yellow-600/20 text-yellow-400 hover:bg-yellow-600/30 text-xs font-medium transition-colors"
                    >
                      Pause
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); stopVm(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-red-600/20 text-red-400 hover:bg-red-600/30 text-xs font-medium transition-colors"
                    >
                      Stop
                    </button>
                  {:else if vm.state === "Paused"}
                    <button
                      onclick={(e) => { e.stopPropagation(); openSpiceDisplay(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-blue-600/20 text-blue-400 hover:bg-blue-600/30 text-xs font-medium transition-colors"
                    >
                      Display
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); resumeVm(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/30 text-xs font-medium transition-colors"
                    >
                      Resume
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); stopVm(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-red-600/20 text-red-400 hover:bg-red-600/30 text-xs font-medium transition-colors"
                    >
                      Stop
                    </button>
                  {:else}
                    <button
                      onclick={(e) => { e.stopPropagation(); startVm(vm.config.id); }}
                      class="flex-1 px-3 py-1.5 rounded-lg bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/30 text-xs font-medium transition-colors"
                    >
                      Start
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); editVm(vm.config.id); }}
                      aria-label="Edit VM"
                      class="px-3 py-1.5 rounded-lg bg-neutral-800 text-neutral-400 hover:bg-neutral-700 text-xs transition-colors"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                      </svg>
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); exportVm(vm.config.id); }}
                      aria-label="Export VM"
                      class="px-3 py-1.5 rounded-lg bg-neutral-800 text-neutral-400 hover:bg-neutral-700 text-xs transition-colors"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                      </svg>
                    </button>
                    <button
                      onclick={(e) => { e.stopPropagation(); deleteVm(vm.config.id); }}
                      aria-label="Delete VM"
                      class="px-3 py-1.5 rounded-lg bg-neutral-800 text-neutral-400 hover:bg-neutral-700 text-xs transition-colors"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 7l-.867 12.142A2 2 0 0116.138 21H7.862a2 2 0 01-1.995-1.858L5 7m5 4v6m4-6v6m1-10V4a1 1 0 00-1-1h-4a1 1 0 00-1 1v3M4 7h16" />
                      </svg>
                    </button>
                  {/if}
                </div>
              </div>
            {/each}
          </div>
        {/if}
      </div>

      {#if selectedVm && getSelectedVm()}
        {@const vm = getSelectedVm()}
        <aside class="w-80 border-l border-neutral-800 bg-neutral-900/30 p-4 overflow-auto">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-sm font-medium text-white">VM Details</h3>
            <button
              onclick={() => selectedVm = null}
              aria-label="Close details"
              class="p-1 rounded hover:bg-neutral-800 text-neutral-400"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="space-y-4">
            <div class="p-4 rounded-lg bg-neutral-800/50 border border-neutral-800">
              <h4 class="text-xs text-neutral-500 mb-2">Status</h4>
              <div class="flex items-center gap-2">
                <span class="w-2.5 h-2.5 rounded-full {getStateColor(vm?.state || 'Stopped')}"></span>
                <span class="text-sm font-medium text-white">{vm?.state}</span>
              </div>
              {#if vm?.pid}
                <p class="text-xs text-neutral-500 mt-1">PID: {vm?.pid}</p>
              {/if}
            </div>

            {#if vm?.state === "Running" && vmMetrics[vm?.config.id || '']}
              {@const metrics = vmMetrics[vm?.config.id || '']}
              <div class="space-y-3">
                <h4 class="text-xs text-neutral-500 uppercase tracking-wider">Real-time Metrics</h4>
                
                <div class="p-3 rounded-lg bg-neutral-800/50 border border-neutral-800">
                  <Chart
                    data={cpuHistory}
                    color="#3b82f6"
                    label="CPU Usage"
                    max={100}
                    unit="%"
                    current={metrics?.cpu_percent}
                  />
                </div>

                <div class="p-3 rounded-lg bg-neutral-800/50 border border-neutral-800">
                  <Chart
                    data={memHistory}
                    color="#8b5cf6"
                    label="Memory"
                    max={vm?.config.hardware.memory_mb || 4096}
                    unit="MB"
                    current={metrics?.memory_mb}
                  />
                </div>

                <div class="flex gap-2">
                  <div class="flex-1 p-3 rounded-lg bg-neutral-800/50 border border-neutral-800 text-center">
                    <p class="text-xs text-neutral-500 mb-1">Uptime</p>
                    <p class="text-sm font-mono text-white">{formatUptime(metrics?.uptime_seconds || 0)}</p>
                  </div>
                  <div class="flex-1 p-3 rounded-lg bg-neutral-800/50 border border-neutral-800 text-center">
                    <p class="text-xs text-neutral-500 mb-1">Memory</p>
                    <p class="text-sm font-mono text-white">{formatMemory(metrics?.memory_mb || 0)}</p>
                  </div>
                </div>
              </div>
            {:else if vm?.state === "Running"}
              <div class="p-4 rounded-lg bg-neutral-800/50 border border-neutral-800 text-center">
                <p class="text-sm text-neutral-400">Collecting metrics...</p>
              </div>
            {/if}

            <div class="p-4 rounded-lg bg-neutral-800/50 border border-neutral-800">
              <h4 class="text-xs text-neutral-500 mb-3">Configuration</h4>
              <dl class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <dt class="text-neutral-400">Name</dt>
                  <dd class="text-white">{vm?.config.name}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-neutral-400">Architecture</dt>
                  <dd class="text-white">{vm?.config.architecture.toUpperCase()}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-neutral-400">CPU Cores</dt>
                  <dd class="text-white">{vm?.config.hardware.cpu_cores}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-neutral-400">Memory</dt>
                  <dd class="text-white">{formatMemory(vm?.config.hardware.memory_mb || 0)}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-neutral-400">Disk Size</dt>
                  <dd class="text-white">{vm?.config.storage.disk_size_gb} GB</dd>
                </div>
              </dl>
            </div>
          </div>
        </aside>
      {/if}
    </div>
  </main>
</div>

{#if showCreateModal}
  <CreateVmWizard
    onSelect={(isoPath) => {
      if (isoPath !== undefined) {
        createVm("New VM", 4, 4096, 20, isoPath);
      }
      showCreateModal = false;
    }}
    onClose={() => (showCreateModal = false)}
  />
{/if}

{#if showSettings}
  <Settings onClose={() => (showSettings = false)} />
{/if}

{#if showEditModal && editingVm}
  <EditVmModal
    vm={editingVm.config}
    onClose={() => { showEditModal = false; editingVm = null; }}
    onSave={handleVmUpdated}
  />
{/if}

{#if showSpiceDisplay && spiceVmId}
  <SpiceDisplay
    vmId={spiceVmId}
    spicePort={spicePort}
    onClose={() => { showSpiceDisplay = false; spiceVmId = null; }}
  />
{/if}

<div class="fixed bottom-4 right-4 z-50 flex flex-col gap-2">
  {#each toasts as toast (toast.id)}
    <div
      class="px-4 py-3 rounded-lg shadow-lg flex items-center gap-3 min-w-72 max-w-md animate-slide-in {toast.type === 'success' ? 'bg-emerald-900/90 border border-emerald-700' : toast.type === 'error' ? 'bg-red-900/90 border border-red-700' : 'bg-neutral-800/90 border border-neutral-700'}"
      role="alert"
    >
      {#if toast.type === 'success'}
        <svg class="w-5 h-5 text-emerald-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
      {:else if toast.type === 'error'}
        <svg class="w-5 h-5 text-red-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      {:else}
        <svg class="w-5 h-5 text-blue-400 shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      {/if}
      <span class="text-sm text-white">{toast.message}</span>
      <button
        onclick={() => { toasts = toasts.filter(t => t.id !== toast.id); }}
        class="ml-auto text-neutral-400 hover:text-white"
        aria-label="Dismiss"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  {/each}
</div>

{#if cmdPaletteOpen}
  <div class="fixed inset-0 z-50 flex items-start justify-center pt-32">
    <button aria-label="Close command palette" class="absolute inset-0 bg-black/60 backdrop-blur-sm" onclick={() => (cmdPaletteOpen = false)}></button>
    <div class="relative w-full max-w-lg bg-neutral-900 border border-neutral-800 rounded-2xl shadow-2xl overflow-hidden">
      <div class="flex items-center gap-3 px-4 py-3 border-b border-neutral-800">
        <svg class="w-5 h-5 text-neutral-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          bind:value={cmdQuery}
          placeholder="Type a command..."
          class="flex-1 bg-transparent text-white placeholder-neutral-500 text-sm focus:outline-none"
        />
        <kbd class="px-2 py-1 rounded bg-neutral-800 text-neutral-500 text-xs">ESC</kbd>
      </div>
      <div class="p-2 max-h-80 overflow-auto">
        <div class="px-3 py-2 text-xs text-neutral-500 font-medium">Quick Actions</div>
        <button onclick={() => { cmdPaletteOpen = false; showCreateModal = true; }} class="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left text-sm">
          <svg class="w-4 h-4 text-neutral-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
          </svg>
          Create New VM
        </button>
        <button class="w-full flex items-center gap-3 px-3 py-2 rounded-lg hover:bg-neutral-800 text-left text-sm">
          <svg class="w-4 h-4 text-neutral-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
          </svg>
          View All VMs
        </button>
      </div>
    </div>
  </div>
{/if}
