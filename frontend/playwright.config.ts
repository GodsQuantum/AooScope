import { defineConfig } from '@playwright/test';

export default defineConfig({
  testDir: './tests',
  webServer: { command: 'pnpm run build && pnpm exec vite preview --host 127.0.0.1', port: 4173, reuseExistingServer: true },
  use: { baseURL: 'http://127.0.0.1:4173' }
});
