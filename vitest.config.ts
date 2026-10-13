import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}", "scripts/**/*.test.ts"],
    // Stylesheets stay empty modules in tests, except the two the
    // ConditionArea test reads as text (`?raw`) to check that the date field
    // is as wide as Cloudscape's DatePicker allows (gate fix 14a).
    css: { include: [/styles\.css\?raw$/, /date-picker\/styles\.scoped\.css\?raw$/] },
    // Line coverage of the screen code, measured by `npm run test:coverage`
    // (CI). Below 80% the run fails (team.md Testing Posture).
    coverage: {
      provider: "v8",
      include: ["src/**/*.{ts,tsx}"],
      exclude: ["src/**/*.test.{ts,tsx}", "src/test/**"],
      reporter: ["text-summary"],
      thresholds: { lines: 80 },
    },
  },
});
