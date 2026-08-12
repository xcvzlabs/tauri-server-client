<script setup lang="ts">
  import type { UserProfile } from '~/types/api.ts';

  const session = useSession();
  const presence = usePresence();
  const hosting = useHosting();

  const users = ref<UserProfile[]>([]);
  const isLoading = ref(true);
  const errorMessage = ref<string | null>(null);

  async function loadUsers(): Promise<void> {
    isLoading.value = true;
    errorMessage.value = null;
    try {
      users.value = await apiFetch<UserProfile[]>('/api/v1/users');
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to load users';
    } finally {
      isLoading.value = false;
    }
  }

  onMounted(loadUsers);

  function isOnline(userId: string): boolean {
    return presence.onlineUserIds.value.has(userId);
  }

  async function handleLogout(): Promise<void> {
    await session.logout();
    await navigateTo('/login');
  }

  async function handleStopHosting(): Promise<void> {
    await hosting.stop();
    await session.logout();
    await navigateTo('/');
  }
</script>

<template>
  <main class="screen">
    <div class="card wide">
      <header class="home-header">
        <div>
          <h1>
            {{ session.currentUser.value?.display_name }}
            <span
              v-if="session.currentUser.value"
              class="role-badge"
              :class="{ admin: session.currentUser.value.role === 'admin' }"
            >
              {{ session.currentUser.value.role }}
            </span>
          </h1>
          <p class="hint">{{ session.currentUser.value?.email }}</p>
        </div>
        <div class="home-actions">
          <NuxtLink
            v-if="session.currentUser.value?.role === 'admin'"
            to="/admin"
          >
            Manage users
          </NuxtLink>
          <button
            v-if="hosting.isHosting.value"
            type="button"
            @click="handleStopHosting"
          >
            Stop hosting
          </button>
          <button
            type="button"
            @click="handleLogout"
          >
            Log out
          </button>
        </div>
      </header>

      <p
        v-if="errorMessage"
        class="error"
      >
        {{ errorMessage }}
      </p>
      <p
        v-else-if="isLoading"
        class="hint"
      >
        Loading users…
      </p>

      <ul
        v-else
        class="user-list"
      >
        <li
          v-for="user in users"
          :key="user.id"
          class="user-row"
        >
          <span
            class="dot"
            :class="{ online: isOnline(user.id) }"
          />
          <span class="username">{{ user.display_name }}</span>
          <span class="handle">@{{ user.username }}</span>
          <span
            v-if="user.role === 'admin'"
            class="role-badge admin"
          >
            admin
          </span>
        </li>
      </ul>
    </div>
  </main>
</template>
