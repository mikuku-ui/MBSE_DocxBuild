import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

// 本地开发代理：/api → 后端（后端配置见仓库根 config.toml，端口 38123）。
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    port: 5173,
    proxy: {
      '/api': {
        target: 'http://127.0.0.1:38123',
        changeOrigin: true,
      },
    },
  },
});
