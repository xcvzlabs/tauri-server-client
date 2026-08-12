<script setup lang="ts">
  const session = useSession();
  const hosting = useHosting();

  const isStarting = ref(false);
  const errorMessage = ref<string | null>(null);

  async function handleHost(): Promise<void> {
    errorMessage.value = null;
    isStarting.value = true;
    try {
      const info = await hosting.start();
      await session.setServerUrl(`${info.host}:${info.port}`);
      // If a session was already restored for this machine's server, the global
      // middleware will have already sent us to /home before this even runs. Reaching
      // here means there's no account yet, so register the initial admin account.
      await navigateTo('/register');
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to start hosting';
    } finally {
      isStarting.value = false;
    }
  }

  async function handleJoin(): Promise<void> {
    await navigateTo('/connect');
  }
</script>

<template>
  <main class="screen">
    <div class="card">
      <h1>Welcome</h1>
      <p class="hint">Are you hosting this session, or joining one that's already running?</p>

      <button
        type="button"
        :disabled="isStarting"
        @click="handleHost"
      >
        {{ isStarting ? 'Starting server…' : "Host (I'm the admin)" }}
      </button>
      <button
        type="button"
        @click="handleJoin"
      >
        Join a server
      </button>

      <p
        v-if="errorMessage"
        class="error"
      >
        {{ errorMessage }}
      </p>
    </div>
  </main>
</template>
