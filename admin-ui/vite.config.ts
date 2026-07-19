import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

export default defineConfig({
  plugins: [tailwindcss(), svelte()],
  base: "/",
  build: {
    outDir: "build",
    emptyOutDir: true,
    // Warn locally if a single chunk regresses (e.g. lost view code-splitting).
    // A hard budget is enforced in CI.
    chunkSizeWarningLimit: 175
  }
});
