/**
 * Tauri mock for browser-based Playwright testing.
 *
 * The @tauri-apps/api modules call `window.__TAURI_INTERNALS__.invoke(cmd, args)`
 * and `window.__TAURI_INTERNALS__.transformCallback(cb)`. This script injects
 * those so the app runs inside a normal browser.
 *
 * Commands are dispatched to a registry so tests can stub responses. Events
 * registered via `plugin:event|listen` are fired with
 * `window.__TAURI_EMIT_EVENT__(event, payload)`.
 */

import type { Page } from "@playwright/test";

export interface MockVmConfig {
  name: string;
  id: string;
  architecture: string;
  hardware: { cpu_cores: number; memory_mb: number };
  storage: { disk_size_gb: number };
  network: { enabled: boolean; mode: string };
  display: { resolution: { width: number; height: number } };
}

export interface MockVm {
  config: MockVmConfig;
  state: string;
  pid: number | null;
}

declare global {
  interface Window {
    __TAURI_MOCK__: {
      setVms(vms: MockVm[]): void;
      getVms(): MockVm[];
      setHandler(cmd: string, fn: (args: any) => any): void;
      calls(): Array<{ cmd: string; args: any }>;
      savedSettings(): any;
      emit(event: string, payload?: any): void;
      clear(): void;
    };
    __TAURI_EMIT_EVENT__(event: string, payload?: any): void;
  }
}

export const INIT_SCRIPT = `
(function () {
  if (window.__TAURI_MOCK__) return;

  var handlers = {};
  var eventCallbacks = {};
  var callLog = [];
  var msgId = 1;
  var vms = [];

  function genId() { return "cb_" + (msgId++); }

  function transformCallback(cb, once) {
    var id = genId();
    window[id] = function (payload) {
      try { once ? (delete window[id], cb(payload)) : cb(payload); }
      catch (err) { console.error(err); }
    };
    return id;
  }

  function invoke(cmd, args) {
    callLog.push({ cmd: cmd, args: args || {} });
    return new Promise(function (resolve, reject) {
      var handler = handlers[cmd];
      if (!handler) {
        reject(new Error("no mock handler for command: " + cmd));
        return;
      }
      try {
        var out = handler(args || {});
        if (out && typeof out.then === "function") {
          out.then(resolve, function (e) { reject(new Error(String(e))); });
        } else {
          resolve(out);
        }
      } catch (err) {
        reject(new Error(err && err.message ? err.message : String(err)));
      }
    });
  }

  function listen(event, handler, options) {
    var handlerId = transformCallback(handler, false);
    invoke("plugin:event|listen", { event: event, target: (options && options.target) || { kind: "Any" }, handlerId: handlerId })
      .then(function () {
        eventCallbacks[event] = eventCallbacks[event] || [];
        eventCallbacks[event].push(handlerId);
      })
      .catch(function () {});
    return Promise.resolve(function () {
      invoke("plugin:event|unlisten", { eventId: handlerId }).catch(function () {});
      if (eventCallbacks[event]) {
        eventCallbacks[event] = eventCallbacks[event].filter(function (h) { return h !== handlerId; });
      }
    });
  }

  window.__TAURI_INTERNALS__ = {
    transformCallback: transformCallback,
    invoke: invoke,
    listen: listen,
    metadata: { currentWindow: { label: "main" }, currentWebview: { label: "main" } },
    convertFileSrc: function (p) { return p; }
  };

  window.__TAURI__ = {
    core: { invoke: invoke },
    event: { listen: listen }
  };

  window.__TAURI_EMIT_EVENT__ = function (event, payload) {
    (eventCallbacks[event] || []).forEach(function (id) {
      var fn = window[id];
      if (fn) fn({ event: event, id: 0, payload: payload });
    });
  };

  // ---- default command handlers -------------------------------------------
  handlers["plugin:event|listen"] = function () { return 1; };
  handlers["plugin:event|unlisten"] = function () { return null; };
  handlers["plugin:event|emit"] = function (args) {
    window.__TAURI_EMIT_EVENT__(args.event, args.payload);
    return null;
  };

  handlers["list_vms"] = function () { return vms; };
  handlers["create_vm"] = function (args) {
    var vm = {
      config: {
        name: args.name,
        id: (args.name || "vm").toLowerCase().replace(/[^a-z0-9]+/g, "-") + "-" + (vms.length + 1),
        architecture: "aarch64",
        hardware: { cpu_cores: Number(args.cpuCores) || 4, memory_mb: Number(args.memoryMb) || 4096 },
        storage: { disk_size_gb: Number(args.diskSizeGb) || 20 },
        network: { enabled: true, mode: "nat" },
        display: { resolution: { width: 1920, height: 1080 } }
      },
      state: "Stopped",
      pid: null
    };
    vms.push(vm);
    return vm.config;
  };
  handlers["start_vm"] = function (args) { return setState(args.vmId, "Running"); };
  handlers["stop_vm"] = function (args) { return setState(args.vmId, "Stopped"); };
  handlers["pause_vm"] = function (args) { return setState(args.vmId, "Paused"); };
  handlers["resume_vm"] = function (args) { return setState(args.vmId, "Running"); };
  handlers["delete_vm"] = function (args) {
    vms = vms.filter(function (v) { return v.config.id !== args.vmId; });
    return null;
  };
  handlers["update_vm"] = function (args) {
    var vm = find(args.vmId);
    if (vm) {
      vm.config.name = args.name;
      vm.config.hardware.cpu_cores = Number(args.cpuCores);
      vm.config.hardware.memory_mb = Number(args.memoryMb);
      vm.config.storage.disk_size_gb = Number(args.diskSizeGb);
      vm.config.network.enabled = !!args.networkEnabled;
      vm.config.network.mode = args.networkMode;
      vm.config.display.resolution = { width: Number(args.displayWidth), height: Number(args.displayHeight) };
    }
    return null;
  };
  handlers["export_vm"] = function () { return null; };
  handlers["import_vm"] = function (args) {
    var vm = {
      config: {
        name: "Imported VM",
        id: "imported-" + (vms.length + 1),
        architecture: "aarch64",
        hardware: { cpu_cores: 4, memory_mb: 4096 },
        storage: { disk_size_gb: 20 },
        network: { enabled: true, mode: "nat" },
        display: { resolution: { width: 1920, height: 1080 } }
      },
      state: "Stopped",
      pid: null
    };
    vms.push(vm);
    return vm.config;
  };
  handlers["get_vm_metrics"] = function (args) {
    return { vm_id: args.vmId, cpu_percent: 12.5, memory_mb: 512, uptime_seconds: 75 };
  };
  handlers["get_all_metrics"] = function () {
    var out = {};
    vms.forEach(function (v) {
      out[v.config.id] = { vm_id: v.config.id, cpu_percent: 12.5, memory_mb: 512, uptime_seconds: 75 };
    });
    return out;
  };
  handlers["get_spice_port"] = function () { return 5900; };
  handlers["get_spice_credentials"] = function () { return { username: "vibevisor", password: "test" }; };
  handlers["get_cpu_topology"] = function () {
    return { performance_cores: 6, efficiency_cores: 2, total_cores: 8 };
  };
  handlers["suggest_core_allocation"] = function (args) {
    return { recommended_cores: 2, performance_cores: 2, efficiency_cores: 0 };
  };
  handlers["get_settings"] = function () { return settings(); };
  handlers["get_default_settings"] = function () { return settings(); };
  handlers["save_settings"] = function (args) { lastSavedSettings = args.settings; return null; };
  handlers["confirm_and_quit"] = function () { return null; };
  handlers["plugin:dialog|open"] = function () { return "/Users/test/Downloads/alpine.iso"; };
  handlers["plugin:dialog|save"] = function () { return "/Users/test/Downloads/export.tar.gz"; };
  handlers["plugin:dialog|message"] = function (args) {
    var buttons = args.buttons || (args.options || {}).buttons || "Ok";
    if (typeof buttons === "object") {
      if (buttons.OkCancelCustom && buttons.OkCancelCustom.length) return buttons.OkCancelCustom[0];
      if (buttons.ok) return buttons.ok;
      return "Ok";
    }
    if (buttons === "YesNo") return "Yes";
    if (buttons === "OkCancel") return "Ok";
    return "Ok";
  };
  handlers["plugin:dialog|ask"] = function () { return true; };
  handlers["plugin:dialog|confirm"] = function () { return true; };

  var lastSavedSettings = null;
  function settings() {
    return {
      default_cpu_cores: 4,
      default_memory_mb: 4096,
      default_disk_gb: 20,
      theme: "dark",
      qemu_custom_path: null,
      auto_start_last_vm: false,
      minimize_to_tray: true
    };
  }
  function find(id) {
    for (var i = 0; i < vms.length; i++) if (vms[i].config.id === id) return vms[i];
    return null;
  }
  function setState(id, next) {
    var vm = find(id);
    if (!vm) return null;
    vm.state = next;
    vm.pid = next === "Running" || next === "Paused" ? 4242 : null;
    return null;
  }

  window.__TAURI_MOCK__ = {
    setVms: function (next) { vms = next; },
    getVms: function () { return vms; },
    setHandler: function (cmd, fn) { handlers[cmd] = fn; },
    calls: function () { return callLog; },
    savedSettings: function () { return lastSavedSettings; },
    emit: function (event, payload) { window.__TAURI_EMIT_EVENT__(event, payload); },
    clear: function () { callLog = []; }
  };
})();
`;

/** Build a VM fixture. */
export function makeVm(overrides: {
  name?: string;
  id?: string;
  state?: string;
  cpu?: number;
  memory?: number;
  disk?: number;
  pid?: number | null;
} = {}): MockVm {
  const name = overrides.name ?? "Ubuntu ARM";
  return {
    config: {
      name,
      id: overrides.id ?? name.toLowerCase().replace(/[^a-z0-9]+/g, "-"),
      architecture: "aarch64",
      hardware: { cpu_cores: overrides.cpu ?? 4, memory_mb: overrides.memory ?? 4096 },
      storage: { disk_size_gb: overrides.disk ?? 20 },
      network: { enabled: true, mode: "nat" },
      display: { resolution: { width: 1920, height: 1080 } },
    },
    state: overrides.state ?? "Stopped",
    pid: overrides.pid ?? null,
  };
}

export const SAMPLE_VMS: MockVm[] = [
  makeVm({ name: "Ubuntu ARM", id: "ubuntu-arm", state: "Running", pid: 4242 }),
  makeVm({ name: "Debian Test", id: "debian-test", state: "Paused", pid: 4243, cpu: 2, memory: 2048 }),
  makeVm({ name: "Fedora Rawhide Builder", id: "fedora", state: "Stopped", disk: 64 }),
];

/** Register the mock (and optionally seed VMs) before the app boots. */
export async function installTauriMock(page: Page, vms: MockVm[] = []) {
  await page.addInitScript(`${INIT_SCRIPT}\nwindow.__TAURI_MOCK__.setVms(${JSON.stringify(vms)});`);
}

/** Wait for the Svelte app to finish hydrating. */
export async function openApp(page: Page) {
  await page.goto("/");
  await page.waitForFunction(() => !!document.querySelector("aside"));
}

export async function lastSavedSettings(page: Page) {
  return page.evaluate(() => (window as any).__TAURI_MOCK__.savedSettings());
}

export async function mockCalls(page: Page) {
  return page.evaluate(() => (window as any).__TAURI_MOCK__.calls());
}
