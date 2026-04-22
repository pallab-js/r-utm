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
  <div class="w-full max-w-lg bg-[#171717] border border-[#2e2e2e] rounded-[16px] overflow-hidden">
    <div class="p-6 border-b border-[#2e2e2e]">
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-[8px] bg-[#3ecf8e]/20 flex items-center justify-center">
          <svg class="w-5 h-5 text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
          </svg>
        </div>
        <div>
          <h2 class="text-lg font-medium text-white">Create New VM</h2>
          <p class="text-sm text-[#898989]">Step {step === "boot" ? "1" : "2"} of 2</p>
        </div>
      </div>
    </div>

    {#if step === "boot"}
      <div class="p-6 space-y-6">
        <div class="space-y-4">
          <h3 class="text-sm font-medium text-[#b4b4b4]">Boot Source</h3>
          
          <button
            onclick={browseIso}
            class="w-full p-6 rounded-[16px] border-2 border-dashed border-[#363636] hover:border-[#3ecf8e]/50 hover:bg-[#3ecf8e]/5 transition-all text-left group"
          >
            <div class="flex items-center gap-4">
              <div class="w-12 h-12 rounded-[8px] bg-[#2e2e2e] flex items-center justify-center group-hover:bg-[#3ecf8e]/20 transition-colors">
                <svg class="w-6 h-6 text-[#b4b4b4] group-hover:text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12" />
                </svg>
              </div>
              <div class="flex-1">
                <p class="text-sm font-medium text-white">Select ISO File</p>
                <p class="text-xs text-[#898989] mt-0.5">{formatPath(isoPath)}</p>
              </div>
              <svg class="w-5 h-5 text-[#898989]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
              </svg>
            </div>
          </button>

          <div class="relative">
            <div class="absolute inset-0 flex items-center">
              <div class="w-full border-t border-[#2e2e2e]"></div>
            </div>
            <div class="relative flex justify-center">
              <span class="px-4 bg-[#171717] text-xs text-[#898989]">or</span>
            </div>
          </div>

          <button
            onclick={handleBootWithoutIso}
            class="w-full p-4 rounded-[16px] border border-[#2e2e2e] hover:border-[#363636] hover:bg-[#2e2e2e]/50 transition-all text-left"
          >
            <div class="flex items-center gap-3">
              <div class="w-10 h-10 rounded-[8px] bg-[#2e2e2e] flex items-center justify-center">
                <svg class="w-5 h-5 text-[#b4b4b4]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4" />
                </svg>
              </div>
              <div>
                <p class="text-sm font-medium text-white">Empty Disk</p>
                <p class="text-xs text-[#898989]">Create a blank virtual disk</p>
              </div>
            </div>
          </button>
        </div>

        <div class="flex justify-end">
          <button
            onclick={handleContinue}
            disabled={!isoPath}
            class="px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium hover:opacity-90 transition-opacity disabled:opacity-50 disabled:cursor-not-allowed"
          >
            Continue
          </button>
        </div>
      </div>
    {:else}
      <div class="p-6 space-y-4">
        <div>
          <label for="vm-name" class="block text-sm font-medium text-[#b4b4b4] mb-2">VM Name</label>
          <input
            id="vm-name"
            type="text"
            bind:value={newVm.name}
            placeholder="My Virtual Machine"
            class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
          />
        </div>

        <div class="grid grid-cols-2 gap-4">
          <div>
            <label for="vm-cpu" class="block text-sm font-medium text-[#b4b4b4] mb-2">CPU Cores</label>
            <input
              id="vm-cpu"
              type="number"
              bind:value={newVm.cpu_cores}
              min="1"
              max="16"
              class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
          <div>
            <label for="vm-memory" class="block text-sm font-medium text-[#b4b4b4] mb-2">Memory (MB)</label>
            <input
              id="vm-memory"
              type="number"
              bind:value={newVm.memory_mb}
              min="512"
              max="65536"
              step="512"
              class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
        </div>

        <div>
          <label for="vm-disk" class="block text-sm font-medium text-[#b4b4b4] mb-2">Disk Size (GB)</label>
          <input
            id="vm-disk"
            type="number"
            bind:value={newVm.disk_size_gb}
            min="1"
            max="1000"
            class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
          />
        </div>

        {#if isoPath}
          <div class="p-3 rounded-[8px] bg-[#3ecf8e]/10 border border-[rgba(62,207,142,0.3)]">
            <div class="flex items-center gap-2 text-sm text-[#3ecf8e]">
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
            class="px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] text-[#b4b4b4] hover:bg-[#363636] transition-colors"
          >
            Back
          </button>
          <button
            onclick={handleCreate}
            class="px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium hover:opacity-90 transition-opacity"
          >
            Create VM
          </button>
        </div>
      </div>
    {/if}

    <button
      onclick={onClose}
      aria-label="Close"
      class="absolute top-4 right-4 p-2 rounded-[6px] hover:bg-[#2e2e2e] text-[#b4b4b4] transition-colors"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
      </svg>
    </button>
  </div>
</div>