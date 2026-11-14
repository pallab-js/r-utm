import { test, expect, type Page } from "@playwright/test";
import { installTauriMock, openApp, SAMPLE_VMS, makeVm } from "./tauri-mock";

/** Every test asserts a clean console; this helper collects the errors. */
function trackConsoleErrors(page: Page) {
  const errors: string[] = [];
  page.on("console", (m) => {
    if (m.type() === "error") errors.push(m.text());
  });
  page.on("pageerror", (e) => errors.push(String(e)));
  return errors;
}

async function boot(page: Page, vms = SAMPLE_VMS) {
  const errors = trackConsoleErrors(page);
  await installTauriMock(page, vms);
  await openApp(page);
  return errors;
}

test.describe("Virtual machine list", () => {
  test("shows every VM with its hardware summary", async ({ page }) => {
    const errors = await boot(page);

    await expect(page.getByRole("heading", { name: "VibeVisor" })).toBeVisible();
    for (const vm of SAMPLE_VMS) {
      await expect(page.getByText(vm.config.name, { exact: true }).first()).toBeVisible();
    }
    await expect(page.getByText("4 cores").first()).toBeVisible();
    await expect(page.getByText("64 GB")).toBeVisible();
    expect(errors).toEqual([]);
  });

  test("empty state offers a create action", async ({ page }) => {
    await boot(page, []);
    await expect(page.getByRole("heading", { name: "No Virtual Machines" })).toBeVisible();
    await page.getByRole("button", { name: "Create VM" }).click();
    await expect(page.getByRole("dialog", { name: "Create New VM" })).toBeVisible();
  });

  test("shows a loading state instead of a false empty state", async ({ page }) => {
    await installTauriMock(page, SAMPLE_VMS);
    await page.addInitScript(`
      window.__TAURI_MOCK__.setHandler("list_vms", function () {
        var vms = window.__TAURI_MOCK__.getVms();
        return new Promise(function (resolve) { setTimeout(function () { resolve(vms); }, 400); });
      });
    `);
    await openApp(page);
    await expect(page.locator("[aria-busy='true']")).toBeVisible();
    await expect(page.getByText("Ubuntu ARM", { exact: true }).first()).toBeVisible();
  });

  test("explains a failed load and can retry", async ({ page }) => {
    await installTauriMock(page, SAMPLE_VMS);
    await page.addInitScript(`
      window.__TAURI_MOCK__.setHandler("list_vms", function () {
        throw new Error("permission denied");
      });
    `);
    await openApp(page);

    await expect(page.getByRole("heading", { name: "Couldn't load your VMs" })).toBeVisible();
    await expect(page.getByText("permission denied")).toBeVisible();

    await page.evaluate(() => {
      window.__TAURI_MOCK__.setHandler("list_vms", () => window.__TAURI_MOCK__.getVms());
    });
    await page.getByRole("button", { name: "Try again" }).click();
    await expect(page.getByText("Ubuntu ARM", { exact: true }).first()).toBeVisible();
  });

  test("does not overflow horizontally when details are open", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Show details for Ubuntu ARM" }).click();

    for (const size of [
      { width: 1440, height: 900 },
      { width: 1200, height: 800 },
      { width: 900, height: 600 },
    ]) {
      await page.setViewportSize(size);
      const overflow = await page.evaluate(() => ({
        scrollW: document.documentElement.scrollWidth,
        clientW: document.documentElement.clientWidth,
      }));
      expect(overflow.scrollW).toBeLessThanOrEqual(overflow.clientW);
      await expect(page.getByRole("heading", { name: "VM Details" })).toBeVisible();
    }
  });
});

test.describe("Create VM wizard", () => {
  test("keeps the close button inside the dialog", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "New VM" }).click();

    const dialog = await page.getByRole("dialog").boundingBox();
    const close = await page.getByRole("button", { name: "Close", exact: true }).boundingBox();
    expect(close!.x).toBeGreaterThan(dialog!.x + dialog!.width - 80);
    expect(close!.y).toBeLessThan(dialog!.y + 80);
  });

  test("creates the VM with the values the user entered", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "New VM" }).click();
    await page.getByRole("button", { name: "Install from ISO" }).click();

    await expect(page.locator("#create-vm-name")).toBeFocused();
    await page.locator("#create-vm-name").fill("Arch Playground");
    await page.locator("#create-vm-cpu").fill("8");
    await page.locator("#create-vm-memory").fill("8192");
    await page.locator("#create-vm-disk").fill("40");
    await page.getByRole("button", { name: "Create VM", exact: true }).click();

    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "Arch Playground" })).toBeVisible();

    const vms = await page.evaluate(() => window.__TAURI_MOCK__.getVms());
    const created = vms[vms.length - 1].config;
    expect(created.name).toBe("Arch Playground");
    expect(created.hardware).toEqual({ cpu_cores: 8, memory_mb: 8192 });
    expect(created.storage.disk_size_gb).toBe(40);
  });

  test("empty disk path still reaches the configuration step", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "New VM" }).click();
    await page.getByRole("button", { name: "Empty Disk" }).click();
    await expect(page.locator("#create-vm-name")).toBeVisible();
    await page.locator("#create-vm-name").fill("Blank");
    await page.getByRole("button", { name: "Create VM", exact: true }).click();
    await expect(page.getByRole("heading", { name: "Blank" })).toBeVisible();
  });

  test("refuses a blank name and explains why", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "New VM" }).click();
    await page.getByRole("button", { name: "Empty Disk" }).click();
    await page.locator("#create-vm-name").fill("   ");
    await page.getByRole("button", { name: "Create VM", exact: true }).click();

    await expect(page.getByText("A name is required.")).toBeVisible();
    const vms = await page.evaluate(() => window.__TAURI_MOCK__.getVms());
    expect(vms).toHaveLength(SAMPLE_VMS.length);
  });

  test("closes with Escape and with the backdrop", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "New VM" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);

    await page.getByRole("button", { name: "New VM" }).click();
    await expect(page.getByRole("dialog")).toBeVisible();
    await page.mouse.click(12, 12); // outside the panel, on the backdrop
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });
});

test.describe("Destructive actions", () => {
  test("asks before deleting a VM", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Delete VM" }).first().click();

    const asks = await page.evaluate(() =>
      window.__TAURI_MOCK__.calls().filter((c) => c.cmd === "plugin:dialog|message")
    );
    expect(asks).toHaveLength(1);
    expect(asks[0].args.message).toContain("cannot be undone");

    const vms = await page.evaluate(() => window.__TAURI_MOCK__.getVms());
    expect(vms).toHaveLength(SAMPLE_VMS.length - 1);
  });

  test("keeps the VM when the user cancels", async ({ page }) => {
    await installTauriMock(page, SAMPLE_VMS);
    await page.addInitScript(`
      window.__TAURI_MOCK__.setHandler("plugin:dialog|message", function () { return "Cancel"; });
    `);
    await openApp(page);

    await page.getByRole("button", { name: "Delete VM" }).first().click();
    const vms = await page.evaluate(() => window.__TAURI_MOCK__.getVms());
    expect(vms).toHaveLength(SAMPLE_VMS.length);
  });
});

test.describe("Command palette", () => {
  test("focuses the input, filters and runs the active command", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: /Search/ }).click();

    const input = page.getByRole("textbox", { name: "Search commands" });
    await expect(input).toBeFocused();

    await input.fill("debian");
    const items = page.getByRole("dialog").getByRole("button");
    await expect(items).toHaveCount(3);
    await expect(items.first()).toHaveText(/Show details for Debian Test/);

    await input.fill("nothing-matches-this");
    await expect(page.getByText(/No commands match/)).toBeVisible();

    await input.fill("start fedora");
    await page.keyboard.press("Enter");
    await expect(page.getByRole("dialog")).toHaveCount(0);
    await expect(page.getByText("VM started")).toBeVisible();
  });

  test("opens with ⌘K and closes with Escape", async ({ page }) => {
    await boot(page);
    await page.keyboard.press("Meta+k");
    await expect(page.getByRole("dialog", { name: "Command palette" })).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });
});

test.describe("Settings", () => {
  test("loads, saves and closes with Escape", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Settings" }).click();
    await expect(page.getByRole("dialog", { name: "Settings" })).toBeVisible();

    await page.locator("#default-cpu").fill("6");
    await page.getByRole("button", { name: "Save Changes" }).click();
    await expect(page.getByText("Saved")).toBeVisible();

    const saved = await page.evaluate(() => window.__TAURI_MOCK__.savedSettings());
    expect(saved.default_cpu_cores).toBe(6);

    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("reset warns that changes still need saving", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Settings" }).click();
    await page.locator("#default-cpu").fill("12");
    await page.getByRole("button", { name: "Reset to Defaults" }).click();
    await expect(page.locator("#default-cpu")).toHaveValue("4");
    await expect(page.getByText("Defaults restored — save to apply")).toBeVisible();
  });

  test("switches theme and persists the choice", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Settings" }).click();
    const theme = page.locator("#theme");
    await expect(theme).toBeEnabled();
    await theme.selectOption("light");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "light");
    await page.getByRole("button", { name: "Save Changes" }).click();
    await expect(page.getByText("Saved")).toBeVisible();

    const saved = await page.evaluate(() => window.__TAURI_MOCK__.savedSettings());
    expect(saved.theme).toBe("light");

    await page.keyboard.press("Escape");
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });
});

test.describe("Monitoring", () => {
  test("renders live metrics without wedging the app", async ({ page }) => {
    const errors = await boot(page);
    await page.getByRole("button", { name: "Show details for Ubuntu ARM" }).click();

    await expect(page.getByText("REAL-TIME METRICS")).toBeVisible();
    await expect(page.getByText("1m 15s")).toBeVisible();
    await page.waitForTimeout(2500);

    // Charts must actually paint inside their bitmap.
    const inked = await page.evaluate(() =>
      Array.from(document.querySelectorAll("aside canvas")).map((c) => {
        const canvas = c as HTMLCanvasElement;
        const data = canvas.getContext("2d")!.getImageData(0, 0, canvas.width, canvas.height).data;
        let painted = 0;
        for (let i = 3; i < data.length; i += 4) if (data[i] > 100) painted++;
        return { w: canvas.width, h: canvas.height, painted };
      })
    );
    expect(inked).toHaveLength(2);
    for (const chart of inked) {
      expect(chart.w).toBeGreaterThan(0);
      expect(chart.painted).toBeGreaterThan(0);
    }
    expect(errors).toEqual([]);
  });
});

test.describe("Edit VM", () => {
  test("keeps a validation error visible and focuses the field", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Show details for Fedora Rawhide Builder" }).click();
    await page.getByRole("button", { name: "Edit VM" }).click();

    const name = page.locator("#vm-name");
    await expect(name).toHaveValue("Fedora Rawhide Builder");
    await name.fill("");
    await page.getByRole("button", { name: "Save Changes" }).click();

    await expect(page.getByText("A name is required.")).toBeVisible();
    await expect(name).toBeFocused();
    await expect(page.getByRole("alert")).toBeVisible();
  });
});

test.describe("VM display", () => {
  test("fits the viewport and exits with Escape", async ({ page }) => {
    await boot(page);
    await page.getByRole("button", { name: "Display" }).first().click();
    await page.waitForTimeout(600);

    const box = await page.getByRole("application", { name: "VM display" }).boundingBox();
    expect(box!.width).toBe(1440);
    expect(box!.height).toBeLessThanOrEqual(900 - 72 + 1);

    await page.keyboard.press("Escape");
    await expect(page.getByRole("application", { name: "VM display" })).toHaveCount(0);
  });
});

test.describe("Card accessibility", () => {
  test("cards expose a single interactive selection target", async ({ page }) => {
    await boot(page);
    const nested = await page.evaluate(() =>
      Array.from(document.querySelectorAll("[aria-label^='Show details for']")).map((el) => {
        const card = el.parentElement!;
        const buttons = card.querySelectorAll("button");
        return { label: el.getAttribute("aria-label"), buttons: buttons.length };
      })
    );
    expect(nested.length).toBe(SAMPLE_VMS.length);
    for (const card of nested) {
      expect(card.label).toMatch(/^Show details for /);
      expect(card.buttons).toBeGreaterThan(1);
    }

    const fakeButtons = await page.evaluate(() =>
      Array.from(document.querySelectorAll("[role='button']")).map((el) => el.tagName)
    );
    expect(fakeButtons).toEqual([]);
  });

  test("icon-only actions have labels and tooltips", async ({ page }) => {
    await boot(page, [makeVm({ name: "Fedora", state: "Stopped" })]);
    for (const name of ["Edit VM", "Export VM", "Delete VM"]) {
      const btn = page.getByRole("button", { name });
      await expect(btn).toHaveAttribute("title", name);
    }
  });
});

test.describe("Export", () => {
  test("asks for a folder and reports where the archive landed", async ({ page }) => {
    await installTauriMock(page, [makeVm({ name: "Fedora", state: "Stopped" })]);
    await page.addInitScript(`
      window.__TAURI_MOCK__.setHandler("plugin:dialog|open", function () { return "/Users/test/Exports"; });
      window.__TAURI_MOCK__.setHandler("export_vm", function (args) {
        return args.exportPath + "/" + args.vmId + ".tar.gz";
      });
    `);
    await openApp(page);
    await page.getByRole("button", { name: "Export VM" }).click();

    const calls = await page.evaluate(() => window.__TAURI_MOCK__.calls());
    const openCall = calls.find((c) => c.cmd === "plugin:dialog|open");
    expect(openCall!.args.options.directory).toBe(true);
    await expect(page.getByText("/Users/test/Exports/fedora.tar.gz")).toBeVisible();
  });
});
