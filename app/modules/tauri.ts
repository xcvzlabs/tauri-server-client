import * as tauriApp from '@tauri-apps/api/app';
import * as tauriCore from '@tauri-apps/api/core';
import * as tauriHttp from '@tauri-apps/plugin-http';
import * as tauriStore from '@tauri-apps/plugin-store';
import { addImports, defineNuxtModule } from 'nuxt/kit';
import { capitalize } from '../utils/capitalize.ts';

type ModuleOptions = {
  prefix: false | string;
};

const tauriModules = [
  { module: tauriApp, prefix: 'App', importPath: '@tauri-apps/api/app' },
  { module: tauriCore, prefix: 'Core', importPath: '@tauri-apps/api/core' },
  { module: tauriHttp, prefix: 'HTTP', importPath: '@tauri-apps/plugin-http' },
  { module: tauriStore, prefix: 'Store', importPath: '@tauri-apps/plugin-store' },
];

export default defineNuxtModule<ModuleOptions>({
  meta: {
    name: 'nuxt-tauri',
    configKey: 'tauri',
  },
  defaults: {
    prefix: 'useTauri',
  },
  setup: (options) => {
    for (const { module, prefix, importPath: from } of tauriModules) {
      const as = `${options.prefix}${prefix}`;

      if ('default' in module) {
        addImports({ name: 'default', from, as });
      }

      for (const name of Object.keys(module)) {
        if (name === 'default') continue;
        addImports({ name, from, as: as + capitalize(name) });
      }
    }
  },
});
