<script setup lang="ts">
  import * as v from 'valibot';

  const loginSchema = v.object({
    usernameOrEmail: v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter your username or email')),
    password: v.pipe(v.string(), v.minLength(1, 'Enter your password')),
  });

  const usernameOrEmail = ref('');
  const password = ref('');
  const errorMessage = ref<string | null>(null);
  const isSubmitting = ref(false);

  const session = useSession();

  async function handleSubmit(): Promise<void> {
    errorMessage.value = null;
    const result = v.safeParse(loginSchema, {
      usernameOrEmail: usernameOrEmail.value,
      password: password.value,
    });
    if (!result.success) {
      errorMessage.value = result.issues[0]?.message ?? 'Check the form and try again';
      return;
    }

    isSubmitting.value = true;
    try {
      await session.login(result.output);
      await navigateTo('/');
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Login failed';
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
      <h1>Log in</h1>

      <label for="username-or-email">Username or email</label>
      <input
        id="username-or-email"
        v-model="usernameOrEmail"
        type="text"
        autofocus
      />

      <label for="password">Password</label>
      <input
        id="password"
        v-model="password"
        type="password"
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
        {{ isSubmitting ? 'Logging in…' : 'Log in' }}
      </button>

      <p class="switch">Don't have an account? <NuxtLink to="/register">Register</NuxtLink></p>
    </form>
  </main>
</template>
