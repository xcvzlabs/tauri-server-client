<template>
  <Button
    variant="ghost"
    class="group p-0"
    type="button"
  >
    <div class="relative size-full">
      <div class="flex size-full items-center border px-2">
        <slot />
      </div>

      <CustomCornerMarks reactive />
    </div>
  </Button>
</template>
