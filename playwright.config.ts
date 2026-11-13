import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  testMatch: /.*\.test\.ts/,
  timeout: 30000,
  fullyParallel: false,
  forbidOnly: !!process.env.CI,
  retries: 0,
  workers: 1,
  reporter: [["list"]],
  use: {
    baseURL: "http://localhost:1420",
    trace: "on-first-retry",
    screenshot: "only-on-failure",
    viewport: { width: 1440, height: 900 },
  },
  webServer: {
    command: "npm run dev",
    url: "http://localhost:1420",
    timeout: 90000,
    reuseExistingServer: true,
  },
});
