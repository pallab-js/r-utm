<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";

  interface Props {
    onSelect: (isoPath: string | null) => void;
    onClose: () => void;
  }

  let { onSelect, onClose }: Props = $props();

  let isoPath = $state<string | null>(null);
  let step = $state<"boot" | "config">("boot");

  let newVm = $state({
    name: "New VM",
    cpu_cores: 4,
    memory_mb: 4096,
    disk_size_gb: 20,
  });

  async function browseIso() {
    try {
      const selected = await open({
        multiple: false,
        filters: [{
          name: "ISO Images",
          extensions: ["iso", "img"]
        }]
      });
      if (selected) {
        isoPath = selected as string;
      }
    } catch (e) {
      console.error("Failed to open file dialog:", e);
    }
  }

  function handleBootWithoutIso() {
    isoPath = null;
    onSelect(null);
  }

  function handleContinue() {
    if (!isoPath && !newVm.name.trim()) {
      return;
    }
    step = "config";
  }

  function handleCreate() {
    onSelect(isoPath);
  }

  function formatPath(path: string | null): string {
    if (!path) return "No ISO selected";
    const parts = path.split("/");
    if (parts.length > 3) {
      return ".../" + parts.slice(-3).join("/");
    }
    return path;
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
  <div class="w-full max-w-lg bg-neutral-900 border border-neutral-800 rounded-2xl shadow-2xl overflow-hidden">
    <div class="p-6 border-b border-neutral-800">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-lg bg-blue-600/20 flex items-center justify-center">
          <svg class="w-5 h-5 text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
          </svg>
        </div>
        <div>
          <h2 class="text-lg font-semibold text-white">Create New VM</h2>
          <p class="text-sm text-neutral-500">Step {step === "boot" ? "1" : "2"} of 2</p>
        </div>
      </div>
    </div>

    {#if step === "boot"}
      <div class="p-6 space-y-6">
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-neutral-300">Boot Source</h3>
          
          <button
            onclick={browseIso}
            class="w-full p-6 rounded-xl border-2 border-dashed border-neutral-700 hover:border-blue-500/50 hover:bg-blue-500/5 transition-all text-left group"
          >
            <div class="flex items-center gap-4">
              <div class="w-12 h-12 rounded-lg bg-neutral-800 flex items-center justify-center group-hover:bg-blue-600/20 transition-colors">
                <svg class="w-6 h-6 text-neutral-400 group-hover:text-blue-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                </svg>
              </div>
              <div class="flex-1">
                <p class="text-sm font-medium text-white">Select ISO File</p>
                <p class="text-xs text-neutral-500 mt-0.5">{formatPath(isoPath)}</p>
              </div>
              <svg class="w-5 h-5 text-neutral-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
            </div>
          </button>

          <div class="relative">
            <div class="absolute inset-0 flex items-center">
              <div class="w-full border-t border-neutral-800"></div>
            </div>
            <div class="relative flex justify-center">
              <span class="px-4 bg-neutral-900 text-xs text-neutral-500">or</span>
            </div>
          </div>

          <button
            onclick={handleBootWithoutIso}
            class="w-full p-4 rounded-xl border border-neutral-800 hover:border-neutral-700 hover:bg-neutral-800/50 transition-all text-left"
          >
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-lg bg-neutral-800 flex items-center justify-center">
                <svg class="w-5 h-5 text-neutral-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
                </svg>
              </div>
              <div>
                <p class="text-sm font-medium text-white">Empty Disk</p>
                <p class="text-xs text-neutral-500">Create a blank virtual disk</p>
              </div>
            </div>
          </button>
        </div>

        <div class="flex justify-end">
          <button
            onclick={handleContinue}
            disabled={!isoPath}
            class="px-6 py-2.5 rounded-lg bg-blue-600 text-white text-sm font-medium hover:bg-blue-500 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Continue
          </button>
        </div>
      </div>
    {:else}
      <div class="p-6 space-y-4">
        <div>
          <label for="vm-name" class="block text-sm font-medium text-neutral-300 mb-2">VM Name</label>
          <input
            id="vm-name"
            type="text"
            bind:value={newVm.name}
            placeholder="My Virtual Machine"
            class="w-full px-4 py-2.5 rounded-lg bg-neutral-800 border border-neutral-700 text-white text-sm focus:outline-none focus:border-blue-500"
          />
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="vm-cpu" class="block text-sm font-medium text-neutral-300 mb-2">CPU Cores</label>
            <input
              id="vm-cpu"
              type="number"
              bind:value={newVm.cpu_cores}
              min="1"
              max="16"
              class="w-full px-4 py-2.5 rounded-lg bg-neutral-800 border border-neutral-700 text-white text-sm focus:outline-none focus:border-blue-500"
            />
          </div>
          <div>
            <label for="vm-memory" class="block text-sm font-medium text-neutral-300 mb-2">Memory (MB)</label>
            <input
              id="vm-memory"
              type="number"
              bind:value={newVm.memory_mb}
              min="512"
              max="65536"
              step="512"
              class="w-full px-4 py-2.5 rounded-lg bg-neutral-800 border border-neutral-700 text-white text-sm focus:outline-none focus:border-blue-500"
            />
          </div>
        </div>

        <div>
          <label for="vm-disk" class="block text-sm font-medium text-neutral-300 mb-2">Disk Size (GB)</label>
          <input
            id="vm-disk"
            type="number"
            bind:value={newVm.disk_size_gb}
            min="1"
            max="1000"
            class="w-full px-4 py-2.5 rounded-lg bg-neutral-800 border border-neutral-700 text-white text-sm focus:outline-none focus:border-blue-500"
          />
        </div>

        {#if isoPath}
          <div class="p-3 rounded-lg bg-blue-600/10 border border-blue-600/20">
            <div class="flex items-center gap-2 text-sm text-blue-400">
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
              </svg>
              <span class="truncate">{isoPath.split("/").pop()}</span>
            </div>
          </div>
        {/if}

        <div class="flex justify-between pt-4">
          <button
            onclick={() => step = "boot"}
            class="px-4 py-2.5 rounded-lg bg-neutral-800 text-neutral-300 text-sm hover:bg-neutral-700 transition-colors"
          >
            Back
          </button>
          <button
            onclick={handleCreate}
            class="px-6 py-2.5 rounded-lg bg-blue-600 text-white text-sm font-medium hover:bg-blue-500 transition-colors"
          >
            Create VM
          </button>
        </div>
      </div>
    {/if}

    <button
      onclick={onClose}
      aria-label="Close"
      class="absolute top-4 right-4 p-2 rounded-lg hover:bg-neutral-800 text-neutral-400 transition-colors"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>
</div>
