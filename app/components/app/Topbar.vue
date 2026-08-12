<script setup lang="ts">
  import { MoonIcon, SunIcon } from 'lucide-vue-next';

  const colorMode = useColorMode();

  function toggleThemePreference() {
    colorMode.preference = colorMode.value === 'light' ? 'dark' : 'light';
  }
</script>

<template>
  <div
    class="border-muted/50 flex h-9 items-center justify-between gap-1 border-b bg-zinc-50/80 p-1.5 pl-20.25 dark:bg-zinc-950/20"
    data-tauri-drag-region
  >
    <h1 class="text-xs">Tauri Server Client</h1>

    <Button
      size="icon-sm"
      variant="ghost"
      @click="toggleThemePreference"
    >
      <span class="sr-only">Toggle theme</span>

      <MoonIcon class="scale-100 rotate-0 transition-all dark:scale-0 dark:-rotate-90" />
      <SunIcon class="absolute scale-0 rotate-90 transition-all dark:scale-100 dark:rotate-0" />
    </Button>
  </div>
</template>
