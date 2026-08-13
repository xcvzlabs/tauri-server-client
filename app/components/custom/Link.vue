<script setup lang="ts">
  type Props = {
    to: string;
    'aria-label': string;
  };

  const props = defineProps<Props>();
</script>

<template>
  <Button
    v-bind="props"
    variant="ghost"
    class="group p-0"
    as-child
  >
    <NuxtLink class="relative cursor-pointer">
      <div class="flex size-full items-center border px-2">
        <slot />
      </div>

      <CustomCornerMarks reactive />
    </NuxtLink>
  </Button>
</template>
