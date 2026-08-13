<script setup lang="ts">
  const serverAddress = ref<string>('');
  const errorMessage = ref<string | null>(null);
  const isSubmitting = ref<boolean>(false);

  const session = useSession();

  async function handleSubmit(): Promise<void> {
    errorMessage.value = null;
    isSubmitting.value = true;
    try {
      await session.setServerURL(serverAddress.value);
      await navigateTo('/signin');
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : 'Could not reach a server at that address';
    } finally {
      isSubmitting.value = false;
    }
  }
</script>

<template>
  <AppPageContainer class="flex items-center justify-center p-6">
    <CustomCard class="w-full max-w-sm">
      <CardTitle class="mb-1">Connect to a server</CardTitle>
      <CardDescription class="mb-4">
        Enter the address of the server on your LAN, e.g. <code>192.168.1.10:8080</code>.
      </CardDescription>

      <form
        class="flex flex-col gap-4"
        @submit.prevent="handleSubmit"
      >
        <Field :data-invalid="!!errorMessage">
          <FieldLabel for="server-address">Server address</FieldLabel>

          <div class="group relative">
            <Input
              id="server-address"
              v-model="serverAddress"
              :aria-invalid="!!errorMessage"
              type="text"
              placeholder="192.168.1.10:8080"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
              spellcheck="false"
              autofocus
            />

            <CustomCornerMarks focus-within />
          </div>

          <FieldError
            v-if="errorMessage"
            :errors="[{ message: errorMessage }]"
          />
        </Field>

        <CustomButton
          :disabled="isSubmitting"
          type="submit"
        >
          {{ isSubmitting ? 'Connecting…' : 'Connect' }}
        </CustomButton>
      </form>
    </CustomCard>
  </AppPageContainer>
</template>
