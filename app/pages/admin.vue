<script setup lang="ts">
  import type { UserProfile } from '~/types/api.ts';

  const session = useSession();

  const users = ref<UserProfile[]>([]);
  const isLoading = ref(true);
  const errorMessage = ref<string | null>(null);
  const pendingUserId = ref<string | null>(null);

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

  onMounted(async () => {
    if (session.currentUser.value?.role !== 'admin') {
      await navigateTo('/home');
      return;
    }
    await loadUsers();
  });

  async function toggleRole(user: UserProfile): Promise<void> {
    const nextRole = user.role === 'admin' ? 'member' : 'admin';
    errorMessage.value = null;
    pendingUserId.value = user.id;
    try {
      await apiSend(`/api/v1/admin/users/${user.id}/role`, {
        method: 'PATCH',
        body: JSON.stringify({ role: nextRole }),
      });
      await loadUsers();
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to update role';
    } finally {
      pendingUserId.value = null;
    }
  }

  async function removeUser(user: UserProfile): Promise<void> {
    if (
      !window.confirm(`Remove ${user.display_name} (@${user.username})? This cannot be undone.`)
    ) {
      return;
    }

    errorMessage.value = null;
    pendingUserId.value = user.id;
    try {
      await apiSend(`/api/v1/admin/users/${user.id}`, { method: 'DELETE' });
      await loadUsers();
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to remove user';
    } finally {
      pendingUserId.value = null;
    }
  }
</script>

<template>
  <main class="screen">
    <div class="card wide">
      <header class="home-header">
        <h1>Manage users</h1>
        <NuxtLink to="/home">Back</NuxtLink>
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
            :class="{ online: user.is_online }"
          />
          <span class="username">{{ user.display_name }}</span>
          <span class="handle">@{{ user.username }}</span>
          <span
            class="role-badge"
            :class="{ admin: user.role === 'admin' }"
          >
            {{ user.role }}
          </span>
          <div
            v-if="user.id !== session.currentUser.value?.id"
            class="admin-actions"
          >
            <button
              type="button"
              :disabled="pendingUserId === user.id"
              @click="toggleRole(user)"
            >
              {{ user.role === 'admin' ? 'Demote' : 'Promote' }}
            </button>
            <button
              type="button"
              :disabled="pendingUserId === user.id"
              @click="removeUser(user)"
            >
              Remove
            </button>
          </div>
        </li>
      </ul>
    </div>
  </main>
</template>
