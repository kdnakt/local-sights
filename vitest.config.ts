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
  },
});
