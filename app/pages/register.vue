<script setup lang="ts">
  import * as v from 'valibot';

  const registerSchema = v.object({
    username: v.pipe(
      v.string(),
      v.trim(),
      v.minLength(3, 'Must be at least 3 characters'),
      v.maxLength(32, 'Must be at most 32 characters'),
    ),
    email: v.pipe(v.string(), v.trim(), v.email('Enter a valid email address')),
    password: v.pipe(v.string(), v.minLength(8, 'Must be at least 8 characters')),
    displayName: v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter a display name')),
  });

  const username = ref('');
  const email = ref('');
  const password = ref('');
  const displayName = ref('');
  const errorMessage = ref<string | null>(null);
  const isSubmitting = ref(false);

  const session = useSession();

  async function handleSubmit(): Promise<void> {
    errorMessage.value = null;
    const result = v.safeParse(registerSchema, {
      username: username.value,
      email: email.value,
      password: password.value,
      displayName: displayName.value,
    });
    if (!result.success) {
      errorMessage.value = result.issues[0]?.message ?? 'Check the form and try again';
      return;
    }

    isSubmitting.value = true;
    try {
      await session.register(result.output);
      await navigateTo('/');
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Registration failed';
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
      <h1>Create an account</h1>

      <label for="username">Username</label>
      <input
        id="username"
        v-model="username"
        type="text"
        autofocus
      />

      <label for="email">Email</label>
      <input
        id="email"
        v-model="email"
        type="email"
      />

      <label for="display-name">Display name</label>
      <input
        id="display-name"
        v-model="displayName"
        type="text"
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
        {{ isSubmitting ? 'Creating account…' : 'Create account' }}
      </button>

      <p class="switch">Already have an account? <NuxtLink to="/login">Log in</NuxtLink></p>
    </form>
  </main>
</template>
