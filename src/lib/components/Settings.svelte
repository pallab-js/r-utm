<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  interface AppSettings {
    default_cpu_cores: number;
    default_memory_mb: number;
    default_disk_gb: number;
    theme: string;
    qemu_custom_path: string | null;
    auto_start_last_vm: boolean;
    minimize_to_tray: boolean;
  }

  interface Props {
    onClose?: () => void;
  }

  let { onClose }: Props = $props();

  let settings = $state<AppSettings>({
    default_cpu_cores: 4,
    default_memory_mb: 4096,
    default_disk_gb: 20,
    theme: "dark",
    qemu_custom_path: null,
    auto_start_last_vm: false,
    minimize_to_tray: true,
  });

  let saving = $state(false);
  let saved = $state(false);

  onMount(async () => {
    try {
      settings = await invoke<AppSettings>("get_default_settings");
    } catch (e) {
      console.error("Failed to load settings:", e);
    }
  });

  async function saveSettings() {
    saving = true;
    try {
      await invoke("save_settings", { settings });
      saved = true;
      setTimeout(() => { saved = false; }, 2000);
    } catch (e) {
      console.error("Failed to save settings:", e);
    }
    saving = false;
  }

  function resetDefaults() {
    settings = {
      default_cpu_cores: 4,
      default_memory_mb: 4096,
      default_disk_gb: 20,
      theme: "dark",
      qemu_custom_path: null,
      auto_start_last_vm: false,
      minimize_to_tray: true,
    };
  }
</script>

<div class="fixed inset-0 z-50 flex items-center justify-center bg-black/60 backdrop-blur-sm">
  <div class="w-full max-w-2xl bg-[#171717] border border-[#2e2e2e] rounded-[16px] max-h-[90vh] overflow-hidden flex flex-col">
    <div class="p-6 border-b border-[#2e2e2e] flex items-center justify-between shrink-0">
      <div>
        <h2 class="text-xl font-medium text-white">Settings</h2>
        <p class="text-sm text-[#898989] mt-1">Configure VibeVisor preferences</p>
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

    <div class="flex-1 overflow-auto p-6 space-y-8">
      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2V6z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 6a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2V6z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2H6a2 2 0 01-2-2v-2z" />
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M14 16a2 2 0 012-2h2a2 2 0 012 2v2a2 2 0 01-2 2h-2a2 2 0 01-2-2v-2z" />
          </svg>
          Default VM Configuration
        </h3>
        
        <div class="grid grid-cols-3 gap-4">
          <div>
            <label for="default-cpu" class="block text-sm text-[#b4b4b4] mb-2">CPU Cores</label>
            <input
              id="default-cpu"
              type="number"
              bind:value={settings.default_cpu_cores}
              min="1"
              max="16"
              class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
          
          <div>
            <label for="default-memory" class="block text-sm text-[#b4b4b4] mb-2">Memory (MB)</label>
            <input
              id="default-memory"
              type="number"
              bind:value={settings.default_memory_mb}
              min="512"
              max="65536"
              step="512"
              class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
          
          <div>
            <label for="default-disk" class="block text-sm text-[#b4b4b4] mb-2">Disk Size (GB)</label>
            <input
              id="default-disk"
              type="number"
              bind:value={settings.default_disk_gb}
              min="1"
              max="1000"
              class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
            />
          </div>
        </div>
      </section>

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-purple-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9.75 17L9 20l-1 1h8l-1-1-.75-3M3 13h18M5 17h14a2 2 0 002-2V5a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z" />
          </svg>
          QEMU Configuration
        </h3>
        
        <div>
          <label for="qemu-path" class="block text-sm text-[#b4b4b4] mb-2">Custom QEMU Path (optional)</label>
          <input
            id="qemu-path"
            type="text"
            bind:value={settings.qemu_custom_path}
            placeholder="Leave empty to auto-detect"
            class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
          />
          <p class="text-xs text-[#898989] mt-2">Auto-detected: /opt/homebrew/bin/qemu-system-aarch64</p>
        </div>
      </section>

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-amber-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 3v1m0 16v1m9-9h-1M4 12H3m15.364 6.364l-.707-.707M6.343 6.343l-.707-.707m12.728 0l-.707.707M6.343 17.657l-.707.707" />
          </svg>
          Appearance
        </h3>
        
        <div>
          <label for="theme" class="block text-sm text-[#b4b4b4] mb-2">Theme</label>
          <select
            id="theme"
            bind:value={settings.theme}
            class="w-full px-4 py-2.5 rounded-[6px] bg-[#2e2e2e] border border-[#363636] text-white text-sm focus:outline-none focus:border-[#3ecf8e]"
          >
            <option value="dark">Dark (Default)</option>
            <option value="light">Light</option>
            <option value="system">System</option>
          </select>
        </div>
      </section>

      <section>
        <h3 class="text-sm font-medium text-[#b4b4b4] mb-4 flex items-center gap-2">
          <svg class="w-4 h-4 text-[#3ecf8e]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 8v4l3 3m6-3a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          Behavior
        </h3>
        
        <div class="space-y-4">
          <label class="flex items-center justify-between cursor-pointer">
            <div>
              <p class="text-sm text-[#b4b4b4]">Minimize to Tray</p>
              <p class="text-xs text-[#898989]">Keep running in background when window is closed</p>
            </div>
            <div class="relative">
              <input type="checkbox" bind:checked={settings.minimize_to_tray} class="sr-only peer" />
              <div class="w-11 h-6 bg-[#363636] rounded-full peer peer-checked:bg-[#3ecf8e] transition-colors"></div>
              <div class="absolute left-1 top-1 w-4 h-4 bg-white rounded-full transition-transform peer-checked:translate-x-5"></div>
            </div>
          </label>
          
          <label class="flex items-center justify-between cursor-pointer">
            <div>
              <p class="text-sm text-[#b4b4b4]">Auto-start Last VM</p>
              <p class="text-xs text-[#898989]">Automatically start the last used VM on launch</p>
            </div>
            <div class="relative">
              <input type="checkbox" bind:checked={settings.auto_start_last_vm} class="sr-only peer" />
              <div class="w-11 h-6 bg-[#363636] rounded-full peer peer-checked:bg-[#3ecf8e] transition-colors"></div>
              <div class="absolute left-1 top-1 w-4 h-4 bg-white rounded-full transition-transform peer-checked:translate-x-5"></div>
            </div>
          </label>
        </div>
      </section>
    </div>

    <div class="p-6 border-t border-[#2e2e2e] flex items-center justify-between shrink-0 bg-[#171717]/50">
      <button
        onclick={resetDefaults}
        class="px-4 py-2 rounded-[6px] text-[#b4b4b4] hover:text-white hover:bg-[#2e2e2e] text-sm transition-colors"
      >
        Reset to Defaults
      </button>
      
      <div class="flex items-center gap-3">
        {#if saved}
          <span class="text-sm text-[#3ecf8e] flex items-center gap-1">
            <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
            </svg>
            Saved
          </span>
        {/if}
        <button
          onclick={onClose}
          class="px-4 py-2 rounded-[6px] bg-[#2e2e2e] text-[#b4b4b4] hover:bg-[#363636] text-sm transition-colors"
        >
          Cancel
        </button>
        <button
          onclick={saveSettings}
          disabled={saving}
          class="px-8 py-2 rounded-[9999px] bg-[#0f0f0f] border border-[#fafafa] text-[#fafafa] text-sm font-medium hover:opacity-90 transition-opacity disabled:opacity-50"
        >
          {saving ? "Saving..." : "Save Changes"}
        </button>
      </div>
    </div>
  </div>
</div>