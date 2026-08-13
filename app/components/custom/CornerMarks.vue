<script setup lang="ts">
  defineProps<{
    reactive?: boolean;
    focusWithin?: boolean;
  }>();

  const cornerClasses = [
    'top-0 left-0 border-t border-l',
    'top-0 right-0 border-t border-r',
    'right-0 bottom-0 border-r border-b',
    'bottom-0 left-0 border-b border-l',
  ];
</script>

<template>
  <span
    v-for="(cornerClass, index) in cornerClasses"
    :key="index"
    :class="[
      cornerClass,
      reactive && 'group-hover:border-foreground transition-colors',
      focusWithin && 'group-focus-within:border-foreground transition-colors',
    ]"
    class="border-muted-foreground pointer-events-none absolute size-2.5"
  />
</template>
