<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { untrack } from "svelte";

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

  interface Props {
    vm: VmConfig;
    onClose?: () => void;
    onSave?: (vm: VmConfig) => void;
  }

  let { vm, onClose, onSave }: Props = $props();

  // Seed the form fields from the VM prop's initial value. The modal is opened
  // fresh per edit, so capturing the initial value is intentional; untrack()
  // reads it without creating a reactive dependency, avoiding Svelte's
  // `state_referenced_locally` lint.
  let name = $state(untrack(() => vm.name));
  let cpuCores = $state(untrack(() => vm.hardware.cpu_cores));
  let memoryMb = $state(untrack(() => vm.hardware.memory_mb));
  let diskSizeGb = $state(untrack(() => vm.storage.disk_size_gb));
  let networkEnabled = $state(untrack(() => vm.network.enabled));
  let networkMode = $state(untrack(() => vm.network.mode));
  let displayWidth = $state(untrack(() => vm.display.resolution.width));
  let displayHeight = $state(untrack(() => vm.display.resolution.height));

  let saving = $state(false);
  let error = $state<string | null>(null);

  const resolutionPresets = [
    { label: "1920x1080 (Full HD)", width: 1920, height: 1080 },
    { label: "2560x1440 (2K)", width: 2560, height: 1440 },
    { label: "3840x2160 (4K)", width: 3840, height: 2160 },
    { label: "1280x720 (HD)", width: 1280, height: 720 },
    { label: "800x600 (SVGA)", width: 800, height: 600 },
  ];

  function setResolution(width: number, height: number) {
    displayWidth = width;
    displayHeight = height;
  }

  async function handleSave() {
    saving = true;
    error = null;
    try {
      await invoke("update_vm", {
        vmId: vm.id,
        name,
        cpuCores,
        memoryMb,
        diskSizeGb,
        networkEnabled,
        networkMode: networkMode.toLowerCase(),
        displayWidth,
        displayHeight,
      });
      onSave?.({
        ...vm,
        name,
        hardware: { ...vm.hardware, cpu_cores: cpuCores, memory_mb: memoryMb },
        storage: { ...vm.storage, disk_size_gb: diskSizeGb },
        network: { ...vm.network, enabled: networkEnabled, mode: networkMode },
        display: { resolution: { width: displayWidth, height: displayHeight } },
      } as VmConfig);
      onClose?.();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      onClose?.();
    }
  }
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="fixed inset-0 z-50 flex items-center justify-center">
  <button aria-label="Close backdrop" class="absolute inset-0 bg-black/70 backdrop-blur-sm" onclick={onClose}></button>
  
  <div class="relative w-full max-w-2xl max-h-[90vh] overflow-hidden flex flex-col bg-[#171717] border border-[#2e2e2e] rounded-[16px]">
    <div class="p-6 border-b border-[#2e2e2e] flex items-center justify-between shrink-0">
      <div>
        <h2 class="text-xl font-medium text-white">Edit VM</h2>
        <p class="text-sm text-[#898989] mt-1">Configure {vm.name}</p>
      </div>
      <button
        onclick={onClose}
        aria-label="Close"
        class="p-2 rounded-[6px] hover:bg-[#2e2e2e] text-[#b4b4b4] transition-colors"
      >
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>

    <div class="flex-1 overflow-auto p-6 space-y-6">
      {#if error}
        <div class="p-3 rounded-[8px] bg-red-900/50 border border-red-800 text-red-400 text-sm">
          {error}
        </div>
      {/if}

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
          General
        </h3>
        <div class="space-y-4">
          <div>
            <label for="vm-name" class="block text-sm text-[#b4b4b4] mb-2">VM Name</label>
            <input
              id="vm-name"
              type="text"
              bind:value={name}
              class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
        </div>
      </section>

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
          Hardware
        </h3>
        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="cpu-cores" class="block text-sm text-[#b4b4b4] mb-2">CPU Cores</label>
            <input
              id="cpu-cores"
              type="number"
              min="1"
              max="32"
              bind:value={cpuCores}
              class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
          <div>
            <label for="memory" class="block text-sm text-[#b4b4b4] mb-2">Memory (MB)</label>
            <input
              id="memory"
              type="number"
              min="256"
              max="131072"
              step="256"
              bind:value={memoryMb}
              class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
        </div>
        <div class="mt-4">
          <label for="disk-size" class="block text-sm text-[#b4b4b4] mb-2">Disk Size (GB)</label>
          <input
            id="disk-size"
            type="number"
            min="1"
            max="1024"
            bind:value={diskSizeGb}
            class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
          />
        </div>
      </section>

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8.111 16.404a5.5 5.5 0 017.778 0M12 20h.01m-7.08-7.071c3.904-3.905 10.236-3.905 14.14 0M1.394 9.393c5.857-5.857 15.355-5.857 21.213 0" />
          </svg>
          Network
        </h3>
        <div class="space-y-4">
          <label class="flex items-center gap-3 cursor-pointer">
            <input
              type="checkbox"
              bind:checked={networkEnabled}
              class="w-4 h-4 rounded-[6px] border-[#363636] bg-[#2e2e2e] text-[#3ecf8e] focus:ring-[#3ecf8e]"
            />
            <span class="text-sm text-[#b4b4b4]">Enable Network</span>
          </label>
          {#if networkEnabled}
            <div>
              <label for="network-mode" class="block text-sm text-[#b4b4b4] mb-2">Network Mode</label>
              <select
                id="network-mode"
                bind:value={networkMode}
                class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
              >
                <option value="nat">NAT</option>
                <option value="bridged">Bridged</option>
                <option value="shared">Shared (vmnet)</option>
              </select>
            </div>
          {/if}
        </div>
      </section>

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-orange-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
          Display
        </h3>
        <div class="space-y-4">
          <div>
            <span class="block text-sm text-[#b4b4b4] mb-2">Resolution Preset</span>
            <div class="flex flex-wrap gap-2">
              {#each resolutionPresets as preset}
                <button
                  type="button"
                  onclick={() => setResolution(preset.width, preset.height)}
                  class="px-3 py-1.5 rounded-[6px] text-xs font-medium transition-colors {displayWidth === preset.width && displayHeight === preset.height ? 'bg-[#0f0f0f] border border-[#fafafa] text-white' : 'bg-[#2e2e2e] text-[#b4b4b4] hover:bg-[#363636]'}"
                >
                  {preset.label}
                </button>
              {/each}
            </div>
          </div>
          <div class="grid grid-cols-2 gap-4">
            <div>
              <label for="display-width" class="block text-sm text-[#b4b4b4] mb-2">Width</label>
              <input
                id="display-width"
                type="number"
                min="640"
                max="7680"
                bind:value={displayWidth}
                class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
              />
            </div>
            <div>
              <label for="display-height" class="block text-sm text-[#b4b4b4] mb-2">Height</label>
              <input
                id="display-height"
                type="number"
                min="480"
                max="4320"
                bind:value={displayHeight}
                class="w-full px-3 py-2 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
              />
            </div>
          </div>
        </div>
      </section>
    </div>

    <div class="p-6 border-t border-[#2e2e2e] flex justify-end gap-3 shrink-0">
      <button
        onclick={onClose}
        class="px-4 py-2 rounded-[6px] bg-[#2e2e2e] text-[#b4b4b4] hover:bg-[#363636] text-sm font-medium transition-colors"
      >
        Cancel
      </button>
      <button
        onclick={handleSave}
        disabled={saving}
        class="px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium hover:opacity-90 transition-opacity disabled:opacity-50"
      >
        {#if saving}
          Saving...
        {:else}
          Save Changes
        {/if}
      </button>
    </div>
  </div>
</div>