import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Cargo writes the binary into src-tauri/target while the app builds.
      // Watching that tree races the linker and kills the dev server with
      // EBUSY on Windows — and nothing in it is a frontend source anyway.
      ignored: ["**/src-tauri/**"],
    },
  },
  build: {
    target: "chrome120",
  },
});
