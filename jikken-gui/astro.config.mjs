import { defineConfig } from 'astro/config';
import solid from "@astrojs/solid-js";

// https://astro.build/config
export default defineConfig({
    integrations: [solid()],
    server: { port: 1420 },
    base: './',
    vite: {
        ssr: { noExternal: ["monaco-editor"] },
        optimizeDeps: {
            force: true,
            include: ["solid-js", "nanostores", "@nanostores/solid", "js-yaml", "monaco-editor"],
        }
    }
});
