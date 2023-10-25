import { defineConfig } from "vite";

export default defineConfig({
  root: "D:\\Simon\\Documents\\FRI\\IOI\\seminar-01\\seminar-01-source",
  server: {
    watch: {
      usePolling: true,
    }
  }
})
