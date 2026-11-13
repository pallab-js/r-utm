<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount, onDestroy } from "svelte";
  import { open, ask } from "@tauri-apps/plugin-dialog";
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
  let loadingVms = $state(true);
  let loadError = $state<string | null>(null);
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
  let cmdIndex = $state(0);
  let cmdInput = $state<HTMLInputElement | null>(null);
  let busyVm = $state<string | null>(null);
  let closeConfirmationUnlisten: (() => void) | null = null;

  onMount(async () => {
    await loadVms();
    startMetricsPolling();
    
    closeConfirmationUnlisten = await listen<string>("close-confirmation-needed", async (event) => {
      const runningCount = parseInt(event.payload);
      
      const shouldQuit = await ask(
        `${runningCount} VM(s) are currently running. Closing VibeVisor will stop all running VMs. Do you want to continue?`,
        {
          title: "Confirm Close",
          kind: "warning",
          okLabel: "Quit",
          cancelLabel: "Stay"
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

  function showToast(text: string, type: Toast["type"] = "info") {
    const id = ++toastId;
    toasts = [...toasts, { id, message: text, type }];
    setTimeout(() => {
      toasts = toasts.filter(t => t.id !== id);
    }, 4000);
  }

  let cpuHistory = $state<number[]>([]);
  let memHistory = $state<number[]>([]);

  function startMetricsPolling() {
    metricsInterval = setInterval(async () => {
      const vmId = selectedVm;
      if (vmId && vms.find(v => v.config.id === vmId)?.state === "Running") {
        try {
          const metrics = await invoke<VmMetrics | null>("get_vm_metrics", { vmId });
          if (metrics) {
            vmMetrics = { ...vmMetrics, [metrics.vm_id]: metrics };
            cpuHistory = [...cpuHistory, metrics.cpu_percent].slice(-60);
            memHistory = [...memHistory, metrics.memory_mb].slice(-60);
          }
        } catch (e) {
          console.error("Failed to fetch metrics:", e);
        }
      }
    }, 1000);
  }

  async function loadVms() {
    const isInitialLoad = loadingVms;
    try {
      vms = await invoke<VmInfo[]>("list_vms");
      loadError = null;
    } catch (e) {
      loadError = String(e);
      // The inline error state already covers the initial load; only warn
      // about failures that happen while the user is working.
      if (!isInitialLoad) showToast(`Failed to refresh VMs: ${e}`, "error");
    } finally {
      loadingVms = false;
    }
  }

  async function createVm(vm: { name: string; cpu: number; memory: number; disk: number; isoPath: string | null }) {
    try {
      await invoke("create_vm", {
        name: vm.name,
        cpuCores: vm.cpu,
        memoryMb: vm.memory,
        diskSizeGb: vm.disk,
        isoPath: vm.isoPath,
      });
      showCreateModal = false;
      showToast(`Created VM: ${vm.name}`, "success");
      await loadVms();
    } catch (e) {
      showToast(`Failed to create VM: ${e}`, "error");
    }
  }

  async function runVmAction(vmId: string, action: "start_vm" | "stop_vm" | "pause_vm" | "resume_vm") {
    if (busyVm) return;
    busyVm = vmId;
    try {
      await invoke(action, { vmId });
      showToast(
        action === "start_vm" ? "VM started" : action === "stop_vm" ? "VM stopped" : action === "pause_vm" ? "VM paused" : "VM resumed",
        action === "stop_vm" || action === "pause_vm" ? "info" : "success"
      );
      if (action === "stop_vm") {
        delete vmMetrics[vmId];
        if (vmId === selectedVm) {
          cpuHistory = [];
          memHistory = [];
        }
      }
      if (action === "start_vm" && vmId === selectedVm) {
        cpuHistory = [];
        memHistory = [];
      }
      await loadVms();
    } catch (e) {
      showToast(`Failed to ${action.split("_")[0]} VM: ${e}`, "error");
    } finally {
      busyVm = null;
    }
  }

  async function deleteVm(vmId: string) {
    const vm = vms.find(v => v.config.id === vmId);
    if (!vm) return;

    const confirmed = await ask(
      `"${vm.config.name}" and its disk image will be permanently deleted. This cannot be undone.`,
      { title: "Delete virtual machine?", kind: "warning", okLabel: "Delete", cancelLabel: "Cancel" }
    );
    if (!confirmed) return;

    try {
      await invoke("delete_vm", { vmId });
      if (selectedVm === vmId) {
        selectedVm = null;
        cpuHistory = [];
        memHistory = [];
      }
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

      // The backend writes "<vm-id>.tar.gz" into the chosen folder.
      const dir = await open({
        directory: true,
        multiple: false,
        title: "Choose a folder for the export",
      });

      if (typeof dir === "string" && dir) {
        const archivePath = await invoke<string>("export_vm", { vmId, exportPath: dir });
        showToast(`Exported ${vm.config.name} to ${archivePath}`, "success");
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
    if (selectedVm === vmId) {
      selectedVm = null;
      return;
    }
    selectedVm = vmId;
    cpuHistory = [];
    memHistory = [];
  }

  type Command = {
    id: string;
    label: string;
    hint?: string;
    icon: "plus" | "list" | "settings" | "play" | "pause" | "stop" | "import";
    run: () => void;
  };

  let commands = $derived.by<Command[]>(() => {
    const list: Command[] = [
      { id: "create", label: "Create New VM", icon: "plus", run: () => (showCreateModal = true) },
      { id: "import", label: "Import VM from archive…", icon: "import", run: () => importVm() },
      { id: "settings", label: "Open Settings", icon: "settings", run: () => (showSettings = true) },
    ];

    for (const vm of vms) {
      list.push({
        id: `details-${vm.config.id}`,
        label: `Show details for ${vm.config.name}`,
        hint: vm.state,
        icon: "list",
        run: () => {
          selectedVm = vm.config.id;
          cpuHistory = [];
          memHistory = [];
        },
      });
      if (vm.state === "Running") {
        list.push({ id: `pause-${vm.config.id}`, label: `Pause ${vm.config.name}`, icon: "pause", run: () => runVmAction(vm.config.id, "pause_vm") });
        list.push({ id: `stop-${vm.config.id}`, label: `Stop ${vm.config.name}`, icon: "stop", run: () => runVmAction(vm.config.id, "stop_vm") });
      } else if (vm.state === "Paused") {
        list.push({ id: `resume-${vm.config.id}`, label: `Resume ${vm.config.name}`, icon: "play", run: () => runVmAction(vm.config.id, "resume_vm") });
        list.push({ id: `stop-${vm.config.id}`, label: `Stop ${vm.config.name}`, icon: "stop", run: () => runVmAction(vm.config.id, "stop_vm") });
      } else {
        list.push({ id: `start-${vm.config.id}`, label: `Start ${vm.config.name}`, icon: "play", run: () => runVmAction(vm.config.id, "start_vm") });
      }
    }
    return list;
  });

  let filteredCommands = $derived.by(() => {
    const q = cmdQuery.trim().toLowerCase();
    if (!q) return commands;
    return commands.filter(c => c.label.toLowerCase().includes(q));
  });

  function openCommandPalette() {
    cmdQuery = "";
    cmdIndex = 0;
    cmdPaletteOpen = true;
    queueMicrotask(() => cmdInput?.focus());
  }

  function closeCommandPalette() {
    cmdPaletteOpen = false;
    cmdQuery = "";
    cmdIndex = 0;
  }

  function runCommand(cmd: Command) {
    closeCommandPalette();
    cmd.run();
  }

  function handlePaletteKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      cmdIndex = Math.min(cmdIndex + 1, filteredCommands.length - 1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      cmdIndex = Math.max(cmdIndex - 1, 0);
    } else if (e.key === "Enter") {
      e.preventDefault();
      const cmd = filteredCommands[cmdIndex];
      if (cmd) runCommand(cmd);
    } else if (e.key === "Escape") {
      e.preventDefault();
      closeCommandPalette();
    }
  }

  function handleQueryInput() {
    cmdIndex = 0;
  }

  function handleKeydown(e: KeyboardEvent) {
    const modalOpen = showCreateModal || showEditModal || showSettings || showSpiceDisplay;
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === "k") {
      if (modalOpen) return;
      e.preventDefault();
      cmdPaletteOpen ? closeCommandPalette() : openCommandPalette();
    }
    if (e.key === "Escape" && cmdPaletteOpen) {
      closeCommandPalette();
    }
  }

  function formatMemory(mb: number): string {
    if (mb >= 1024) {
      const gb = mb / 1024;
      return `${Number.isInteger(gb) ? gb : gb.toFixed(1)} GB`;
    }
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
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="h-screen w-screen bg-[#0f0f0f] text-[#fafafa] flex overflow-hidden font-sans">
  <aside class="w-64 h-full backdrop-blur-xl bg-[#171717]/80 border-r border-[#2e2e2e] flex flex-col">
    <div class="p-6 border-b border-[#2e2e2e]">
      <h1 class="text-xl font-medium tracking-tight text-white">VibeVisor</h1>
      <p class="text-xs text-[#898989] mt-1">Linux ARM64 VMs on Apple Silicon</p>
    </div>
    
    <nav class="flex-1 p-4 space-y-1">
      <button
        aria-current="page"
        class="w-full flex items-center gap-3 px-3 py-2 rounded-[6px] bg-[#2e2e2e] text-white text-sm"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
        </svg>
        Virtual Machines
      </button>
      <button
        onclick={() => (showSettings = true)}
        class="w-full flex items-center gap-3 px-3 py-2 rounded-[6px] text-[#b4b4b4] hover:bg-[#2e2e2e]/50 hover:text-white text-sm transition-colors"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
        Settings
      </button>
    </nav>
    
    <div class="p-4 border-t border-[#2e2e2e]">
      <button
        onclick={() => (showCreateModal = true)}
        class="w-full flex items-center justify-center gap-2 px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium transition-opacity hover:opacity-90"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
        </svg>
        New VM
      </button>
    </div>
  </aside>

  <main class="flex-1 flex flex-col overflow-hidden">
    <header class="h-14 px-6 flex items-center justify-between border-b border-[#2e2e2e] bg-[#171717]/30">
      <h2 class="text-sm font-medium text-[#b4b4b4]">Virtual Machines</h2>
      <div class="flex items-center gap-3">
        <button
          onclick={() => importVm()}
          class="flex items-center gap-2 px-3 py-1.5 rounded-[6px] bg-[#2e2e2e] hover:bg-[#363636] text-[#b4b4b4] text-xs transition-colors"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
          </svg>
          Import
        </button>
        <button
          onclick={openCommandPalette}
          aria-label="Search and run commands"
          class="flex items-center gap-2 px-3 py-1.5 rounded-[6px] bg-[#2e2e2e] hover:bg-[#363636] text-[#b4b4b4] text-xs transition-colors"
        >
          <svg class="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
          <span>Search...</span>
          <kbd class="ml-2 px-1.5 py-0.5 rounded-[6px] bg-[#363636] text-[#898989] text-xs">⌘K</kbd>
        </button>
      </div>
    </header>

    <div class="flex-1 flex overflow-hidden min-h-0">
      <div class="flex-1 min-w-0 overflow-auto p-6">
        {#if loadingVms}
          <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4" aria-busy="true" aria-label="Loading virtual machines">
            {#each [0, 1, 2] as i (i)}
              <div class="p-4 rounded-[16px] bg-[#171717] border border-[#2e2e2e] animate-pulse">
                <div class="flex items-center gap-3 mb-4">
                  <div class="w-10 h-10 rounded-[8px] bg-[#2e2e2e]"></div>
                  <div class="space-y-2">
                    <div class="h-3 w-32 rounded bg-[#2e2e2e]"></div>
                    <div class="h-2 w-16 rounded bg-[#2e2e2e]"></div>
                  </div>
                </div>
                <div class="space-y-2">
                  <div class="h-2.5 w-full rounded bg-[#2e2e2e]"></div>
                  <div class="h-2.5 w-3/4 rounded bg-[#2e2e2e]"></div>
                  <div class="h-2.5 w-2/3 rounded bg-[#2e2e2e]"></div>
                </div>
              </div>
            {/each}
          </div>
        {:else if loadError}
          <div class="h-full flex flex-col items-center justify-center text-center">
            <div class="w-16 h-16 rounded-[16px] bg-red-500/10 flex items-center justify-center mb-4">
              <svg class="w-8 h-8 text-red-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <h3 class="text-lg font-medium text-white mb-2">Couldn't load your VMs</h3>
            <p class="text-sm text-[#898989] mb-6 max-w-md break-words">{loadError}</p>
            <button
              onclick={() => { loadingVms = true; loadError = null; loadVms(); }}
              class="flex items-center gap-2 px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium transition-opacity hover:opacity-90"
            >
              Try again
            </button>
          </div>
        {:else if vms.length === 0}
          <div class="h-full flex flex-col items-center justify-center text-center">
            <div class="w-16 h-16 rounded-[16px] bg-[#2e2e2e]/50 flex items-center justify-center mb-4">
              <svg class="w-8 h-8 text-[#898989]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M5 12h14M12 5v14" />
              </svg>
            </div>
            <h3 class="text-lg font-medium text-[#b4b4b4] mb-2">No Virtual Machines</h3>
            <p class="text-sm text-[#898989] mb-6 max-w-sm">
              Create your first Linux ARM64 virtual machine to get started.
            </p>
            <button
              onclick={() => (showCreateModal = true)}
              class="flex items-center gap-2 px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium transition-opacity hover:opacity-90"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
              </svg>
              Create VM
            </button>
          </div>
        {:else}
          <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-4">
            {#each vms as vm (vm.config.id)}
              <div
                class="group relative flex flex-col text-left p-4 rounded-[16px] bg-[#171717] border transition-all duration-200 {selectedVm === vm.config.id ? 'border-[#3ecf8e] ring-1 ring-[#3ecf8e]/30' : 'border-[#2e2e2e] hover:border-[#363636]'}"
              >
                <button
                  type="button"
                  onclick={() => selectVm(vm.config.id)}
                  aria-label="Show details for {vm.config.name}"
                  aria-pressed={selectedVm === vm.config.id}
                  class="absolute inset-0 z-0 w-full h-full rounded-[16px] cursor-pointer focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#3ecf8e] focus-visible:ring-offset-2 focus-visible:ring-offset-[#0f0f0f]"
                ></button>

                <div class="pointer-events-none flex-1">
                  <div class="flex items-start justify-between gap-3 mb-3">
                    <div class="flex items-center gap-3 min-w-0">
                      <div class="w-10 h-10 shrink-0 rounded-[8px] bg-[#2e2e2e] flex items-center justify-center">
                        <svg class="w-5 h-5 text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z" />
                        </svg>
                      </div>
                      <div class="min-w-0">
                        <h3 class="font-medium text-white leading-snug line-clamp-2" title={vm.config.name}>{vm.config.name}</h3>
                        <p class="text-xs text-[#898989]">{vm.config.architecture.toUpperCase()}</p>
                      </div>
                    </div>
                    <div class="flex items-center gap-1.5 shrink-0">
                      <span class="w-2 h-2 rounded-full {getStateColor(vm.state)}"></span>
                      <span class="text-xs text-[#898989]">{vm.state}</span>
                    </div>
                  </div>

                  <div class="space-y-2 text-sm">
                    <div class="flex justify-between text-[#b4b4b4]">
                      <span>CPU</span>
                      <span>{vm.config.hardware.cpu_cores} cores</span>
                    </div>
                    <div class="flex justify-between text-[#b4b4b4]">
                      <span>Memory</span>
                      <span>{formatMemory(vm.config.hardware.memory_mb)}</span>
                    </div>
                    <div class="flex justify-between text-[#b4b4b4]">
                      <span>Disk</span>
                      <span>{vm.config.storage.disk_size_gb} GB</span>
                    </div>
                  </div>
                </div>

                <div class="relative z-10 mt-4 pt-4 border-t border-[#2e2e2e] flex gap-2">
                  {#if vm.state === "Running"}
                    <button
                      onclick={() => openSpiceDisplay(vm.config.id)}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-[#3ecf8e]/20 text-[#3ecf8e] hover:bg-[#3ecf8e]/30 text-xs font-medium transition-colors"
                    >
                      Display
                    </button>
                    <button
                      onclick={() => runVmAction(vm.config.id, "pause_vm")}
                      disabled={busyVm === vm.config.id}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-yellow-600/20 text-yellow-400 hover:bg-yellow-600/30 text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Pause
                    </button>
                    <button
                      onclick={() => runVmAction(vm.config.id, "stop_vm")}
                      disabled={busyVm === vm.config.id}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-red-600/20 text-red-400 hover:bg-red-600/30 text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Stop
                    </button>
                  {:else if vm.state === "Paused"}
                    <button
                      onclick={() => openSpiceDisplay(vm.config.id)}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-[#3ecf8e]/20 text-[#3ecf8e] hover:bg-[#3ecf8e]/30 text-xs font-medium transition-colors"
                    >
                      Display
                    </button>
                    <button
                      onclick={() => runVmAction(vm.config.id, "resume_vm")}
                      disabled={busyVm === vm.config.id}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/30 text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Resume
                    </button>
                    <button
                      onclick={() => runVmAction(vm.config.id, "stop_vm")}
                      disabled={busyVm === vm.config.id}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-red-600/20 text-red-400 hover:bg-red-600/30 text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Stop
                    </button>
                  {:else}
                    <button
                      onclick={() => runVmAction(vm.config.id, "start_vm")}
                      disabled={busyVm === vm.config.id}
                      class="flex-1 px-3 py-1.5 rounded-[6px] bg-emerald-600/20 text-emerald-400 hover:bg-emerald-600/30 text-xs font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      Start
                    </button>
                    <button
                      onclick={() => editVm(vm.config.id)}
                      aria-label="Edit VM"
                      title="Edit VM"
                      class="px-3 py-1.5 rounded-[6px] bg-[#2e2e2e] text-[#b4b4b4] hover:bg-[#363636] text-xs transition-colors"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M11 5H6a2 2 0 00-2 2v11a2 2 0 002 2h11a2 2 0 002-2v-5m-1.414-9.414a2 2 0 112.828 2.828L11.828 15H9v-2.828l8.586-8.586z" />
                      </svg>
                    </button>
                    <button
                      onclick={() => exportVm(vm.config.id)}
                      aria-label="Export VM"
                      title="Export VM"
                      class="px-3 py-1.5 rounded-[6px] bg-[#2e2e2e] text-[#b4b4b4] hover:bg-[#363636] text-xs transition-colors"
                    >
                      <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-8l-4-4m0 0L8 8m4-4v12" />
                      </svg>
                    </button>
                    <button
                      onclick={() => deleteVm(vm.config.id)}
                      aria-label="Delete VM"
                      title="Delete VM"
                      class="px-3 py-1.5 rounded-[6px] bg-[#2e2e2e] text-[#b4b4b4] hover:bg-red-600/20 hover:text-red-400 text-xs transition-colors"
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
        <aside class="w-80 shrink-0 border-l border-[#2e2e2e] bg-[#171717]/30 p-4 overflow-auto">
          <div class="flex items-center justify-between mb-4">
            <h3 class="text-sm font-medium text-white">VM Details</h3>
            <button
              onclick={() => selectedVm = null}
              aria-label="Close details"
              class="p-1 rounded-[6px] hover:bg-[#2e2e2e] text-[#b4b4b4]"
            >
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <div class="space-y-4">
            <div class="p-4 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e]">
              <h4 class="text-xs text-[#898989] mb-2">Status</h4>
              <div class="flex items-center gap-2">
                <span class="w-2.5 h-2.5 rounded-full {getStateColor(vm?.state || 'Stopped')}"></span>
                <span class="text-sm font-medium text-white">{vm?.state}</span>
              </div>
              {#if vm?.pid}
                <p class="text-xs text-[#898989] mt-1">PID: {vm?.pid}</p>
              {/if}
            </div>

            {#if vm?.state === "Running" && vmMetrics[vm?.config.id || '']}
              {@const metrics = vmMetrics[vm?.config.id || '']}
              <div class="space-y-3">
                <h4 class="text-xs text-[#898989] uppercase tracking-[1.2px]">Real-time Metrics</h4>
                
                <div class="p-3 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e]">
                  <Chart
                    data={cpuHistory}
                    color="#3ecf8e"
                    label="CPU Usage"
                    max={100}
                    unit="%"
                    current={metrics?.cpu_percent}
                  />
                </div>

                <div class="p-3 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e]">
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
                  <div class="flex-1 p-3 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e] text-center">
                    <p class="text-xs text-[#898989] mb-1">Uptime</p>
                    <p class="text-sm font-mono text-white">{formatUptime(metrics?.uptime_seconds || 0)}</p>
                  </div>
                  <div class="flex-1 p-3 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e] text-center">
                    <p class="text-xs text-[#898989] mb-1">Memory</p>
                    <p class="text-sm font-mono text-white">{formatMemory(metrics?.memory_mb || 0)}</p>
                  </div>
                </div>
              </div>
            {:else if vm?.state === "Running"}
              <div class="p-4 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e] text-center">
                <p class="text-sm text-[#b4b4b4]">Collecting metrics...</p>
              </div>
            {/if}

            <div class="p-4 rounded-[8px] bg-[#2e2e2e]/50 border border-[#2e2e2e]">
              <h4 class="text-xs text-[#898989] mb-3">Configuration</h4>
              <dl class="space-y-2 text-sm">
                <div class="flex justify-between">
                  <dt class="text-[#b4b4b4]">Name</dt>
                  <dd class="text-white">{vm?.config.name}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-[#b4b4b4]">Architecture</dt>
                  <dd class="text-white">{vm?.config.architecture.toUpperCase()}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-[#b4b4b4]">CPU Cores</dt>
                  <dd class="text-white">{vm?.config.hardware.cpu_cores}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-[#b4b4b4]">Memory</dt>
                  <dd class="text-white">{formatMemory(vm?.config.hardware.memory_mb || 0)}</dd>
                </div>
                <div class="flex justify-between">
                  <dt class="text-[#b4b4b4]">Disk Size</dt>
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
    onCreate={(vm) => createVm(vm)}
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

<div
  class="fixed bottom-4 right-4 z-50 flex flex-col gap-2"
  role="region"
  aria-label="Notifications"
>
  {#each toasts as toast (toast.id)}
    <div
      class="px-4 py-3 rounded-[8px] flex items-center gap-3 min-w-72 max-w-md animate-slide-in {toast.type === 'success' ? 'bg-emerald-900/90 border border-emerald-700' : toast.type === 'error' ? 'bg-red-900/90 border border-red-700' : 'bg-[#2e2e2e]/90 border border-[#363636]'}"
      role={toast.type === "error" ? "alert" : "status"}
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
        <svg class="w-5 h-5 text-[#3ecf8e] shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
        </svg>
      {/if}
      <span class="text-sm text-white">{toast.message}</span>
      <button
        onclick={() => { toasts = toasts.filter(t => t.id !== toast.id); }}
        class="ml-auto text-[#b4b4b4] hover:text-white"
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
    <button
      aria-label="Close command palette"
      class="absolute inset-0 bg-black/60 backdrop-blur-sm"
      onclick={closeCommandPalette}
    ></button>
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Command palette"
      class="relative w-full max-w-lg bg-[#171717] border border-[#2e2e2e] rounded-[16px] overflow-hidden shadow-2xl"
    >
      <div class="flex items-center gap-3 px-4 py-3 border-b border-[#2e2e2e] transition-colors focus-within:bg-[#1c1c1c] focus-within:border-[#3ecf8e]/50">
        <svg class="w-5 h-5 text-[#898989]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <input
          type="text"
          bind:this={cmdInput}
          bind:value={cmdQuery}
          oninput={handleQueryInput}
          onkeydown={handlePaletteKeydown}
          placeholder="Type a command…"
          aria-label="Search commands"
          class="flex-1 bg-transparent text-white placeholder-[#898989] text-sm focus:outline-none"
        />
        <kbd class="px-2 py-1 rounded-[6px] bg-[#2e2e2e] text-[#898989] text-xs">ESC</kbd>
      </div>
      <div class="p-2 max-h-80 overflow-auto">
        {#if filteredCommands.length === 0}
          <p class="px-3 py-6 text-sm text-[#898989] text-center">
            No commands match “{cmdQuery}”.
          </p>
        {:else}
          {#each filteredCommands as cmd, i (cmd.id)}
            <button
              onclick={() => runCommand(cmd)}
              class="w-full flex items-center gap-3 px-3 py-2 rounded-[6px] text-left text-sm transition-colors {i === cmdIndex ? 'bg-[#2e2e2e] text-white' : 'text-[#b4b4b4] hover:bg-[#2e2e2e]'}"
            >
              <svg class="w-4 h-4 shrink-0 {i === cmdIndex ? 'text-[#3ecf8e]' : 'text-[#898989]'}" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                {#if cmd.icon === "plus"}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 6v6m0 0v6m0-6h6m-6 0H6" />
                {:else if cmd.icon === "list"}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 10h16M4 14h16M4 18h16" />
                {:else if cmd.icon === "settings"}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
                {:else if cmd.icon === "play"}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14.752 11.168l-3.197-2.132A1 1 0 0010 9.87v4.263a1 1 0 001.555.832l3.197-2.132a1 1 0 000-1.664zM21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                {:else if cmd.icon === "pause"}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 9v6m4-6v6m7-3a9 9 0 11-18 0 9 9 0 0118 0z" />
                {:else if cmd.icon === "stop"}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 10a1 1 0 011-1h4a1 1 0 011 1v4a1 1 0 01-1 1h-4a1 1 0 01-1-1v-4z" />
                {:else}
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16v1a3 3 0 003 3h10a3 3 0 003-3v-1m-4-4l-4 4m0 0l-4-4m4 4V4" />
                {/if}
              </svg>
              <span class="truncate">{cmd.label}</span>
              {#if cmd.hint}
                <span class="ml-auto text-xs text-[#898989] shrink-0">{cmd.hint}</span>
              {/if}
            </button>
          {/each}
        {/if}
      </div>
      <div class="px-4 py-2 border-t border-[#2e2e2e] text-xs text-[#898989] flex items-center gap-4">
        <span>↑↓ to navigate</span>
        <span>↵ to run</span>
        <span class="ml-auto">ESC to close</span>
      </div>
    </div>
  </div>
{/if}
