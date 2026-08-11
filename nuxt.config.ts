const host = process.env.TAURI_HOST;

export default defineNuxtConfig({
  compatibilityDate: '2026-08-11',
  experimental: {
    typedPages: true,
  },
  devtools: {
    enabled: true,
  },
  devServer: {
    host: host ?? '0.0.0.0',
  },
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: {
      strictPort: true,
      ws: host
        ? {
            protocol: 'ws',
            port: 3001,
            host,
          }
        : undefined,
      watch: {
        ignored: ['**/src-tauri/**'],
      },
    },
  },
  app: {
    head: {
      title: 'Tauri Server Client',
      charset: 'utf8',
      viewport: 'width=device-width, initial-scale=1',
      meta: [
        {
          name: 'format-detection',
          content: 'no',
        },
        {
          name: 'viewport',
          content: 'width=device-width, initial-scale=1, maximum-scale=1',
        },
      ],
    },
    pageTransition: {
      name: 'page',
      mode: 'out-in',
    },
    layoutTransition: {
      name: 'layout',
      mode: 'out-in',
    },
  },
  router: {
    options: {
      scrollBehaviorType: 'smooth',
    },
  },
  dir: {
    modules: 'app/modules',
  },
  ssr: false,
  ignore: ['**/src-tauri/**'],
});
