import { defineConfig, createLogger } from "vite";
import { sveltekit } from "@sveltejs/kit/vite";
import { svelteTesting } from "@testing-library/svelte/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Custom logger to filter out Svelte 5 @__PURE__ annotation warnings
const logger = createLogger();
const originalWarn = logger.warn.bind(logger);
logger.warn = (msg, options) => {
  // Skip @__PURE__ annotation warnings from Svelte 5 runes compilation
  // (these are harmless - Rollup just can't use the tree-shaking hint)
  if (msg.includes("@__PURE__") || msg.includes("Can't resolve original location of error")) return;
  originalWarn(msg, options);
};

// https://vite.dev/config/
export default defineConfig(async () => ({
  customLogger: logger,
  plugins: [sveltekit(), svelteTesting()],

  test: {
    environment: "jsdom",
    include: ["src/**/*.test.ts"],
    setupFiles: ["./src/test-setup.ts"],
  },

  // Don't externalize TipTap packages in SSR to avoid "unused import" warnings
  // (these are browser-only but used inside $effect which doesn't run in SSR)
  ssr: {
    noExternal: ["@tiptap/core", "@tiptap/starter-kit", "@tiptap/extension-placeholder"],
  },

  build: {
    rollupOptions: {
      onwarn(warning, warn) {
        // Ignore @__PURE__ annotation warnings from Svelte 5 runes compilation
        // (these are harmless - Rollup just can't use the tree-shaking hint)
        if (warning.message?.includes("@__PURE__")) {
          return;
        }
        warn(warning);
      },
    },
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
  },
}));
