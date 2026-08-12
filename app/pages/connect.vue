<script setup lang="ts">
  const serverAddress = ref('');
  const errorMessage = ref<string | null>(null);
  const isSubmitting = ref(false);

  const session = useSession();

  async function handleSubmit(): Promise<void> {
    errorMessage.value = null;
    isSubmitting.value = true;
    try {
      await session.setServerUrl(serverAddress.value);
      await navigateTo('/login');
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : 'Could not reach a server at that address';
    } finally {
      isSubmitting.value = false;
    }
  }
</script>

<template>
  <main class="screen">
    <form
      class="card"
      @submit.prevent="handleSubmit"
    >
      <h1>Connect to a server</h1>
      <p class="hint">
        Enter the address of the server on your LAN, e.g. <code>192.168.1.10:8080</code>.
      </p>

      <label for="server-address">Server address</label>
      <input
        id="server-address"
        v-model="serverAddress"
        type="text"
        placeholder="192.168.1.10:8080"
        autofocus
      />

      <p
        v-if="errorMessage"
        class="error"
      >
        {{ errorMessage }}
      </p>

      <button
        type="submit"
        :disabled="isSubmitting"
      >
        {{ isSubmitting ? 'Connecting…' : 'Connect' }}
      </button>
    </form>
  </main>
</template>
