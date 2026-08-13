<script setup lang="ts">
  import { Server, Users } from 'lucide-vue-next';

  const session = useSession();
  const hosting = useHosting();

  const isStarting = ref<boolean>(false);
  const errorMessage = ref<string | null>(null);

  async function handleHost(): Promise<void> {
    errorMessage.value = null;
    isStarting.value = true;

    try {
      const info = await hosting.start();
      await session.setServerURL(`${info.host}:${info.port}`);
      await navigateTo('/signup');
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to start hosting';
    } finally {
      isStarting.value = false;
    }
  }
</script>

<template>
  <AppPageContainer class="flex items-center justify-center p-6">
    <CustomCard class="w-full max-w-sm">
      <CardTitle class="mb-1">Welcome</CardTitle>
      <CardDescription class="mb-4">
        Are you hosting this session, or joining one that's already running?
      </CardDescription>

      <div class="flex flex-col gap-2">
        <CustomButton
          :disabled="isStarting"
          @click="handleHost"
        >
          <Server data-icon="inline-start" />
          {{ isStarting ? 'Starting server…' : "Host (I'm the admin)" }}
        </CustomButton>

        <CustomLink
          to="/connect"
          aria-label="Join a server"
        >
          <Users data-icon="inline-start" />
          Join a server
        </CustomLink>

        <FieldError
          v-if="errorMessage"
          :errors="[{ message: errorMessage }]"
        />
      </div>
    </CustomCard>
  </AppPageContainer>
</template>
