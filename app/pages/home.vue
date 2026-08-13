<script setup lang="ts">
  import { LogOut, Settings, Square } from 'lucide-vue-next';

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
      users.value = await fetchFromServer<UserProfile[]>('/api/v1/users');
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to load users';
    } finally {
      isLoading.value = false;
    }
  }

  function isOnline(userId: string): boolean {
    return presence.onlineUserIds.value.has(userId);
  }

  async function handleSignout(): Promise<void> {
    await session.signout();
    await navigateTo('/signin');
  }

  async function handleStopHosting(): Promise<void> {
    await hosting.stop();
    await session.signout();
    await navigateTo('/');
  }

  onMounted(loadUsers);
</script>

<template>
  <AppPageContainer class="flex items-center justify-center p-6">
    <CustomCard class="w-full max-w-lg">
      <div class="flex">
        <div class="flex-1">
          <Badge
            v-if="session.currentUser.value"
            :variant="session.currentUser.value.role === 'admin' ? 'default' : 'secondary'"
          >
            {{ session.currentUser.value.role }}
          </Badge>

          <CardTitle class="mt-2 mb-1">
            {{ session.currentUser.value?.display_name }}
          </CardTitle>

          <CardDescription class="text-foreground/30 dark:text-foreground/25 text-xs">
            {{ session.currentUser.value?.email }}
          </CardDescription>

          <CardDescription class="text-foreground/30 dark:text-foreground/25 text-xs">
            Server: {{ session.serverURL.value?.httpBase }}
          </CardDescription>
        </div>

        <CardAction class="flex items-center gap-2">
          <CustomLink
            v-if="session.currentUser.value?.role === 'admin'"
            to="/admin"
            aria-label="Manage users"
          >
            <Settings />
          </CustomLink>

          <CustomButton
            v-if="hosting.isHosting.value"
            aria-label="Stop hosting"
            @click="handleStopHosting"
          >
            <Square />
          </CustomButton>

          <CustomButton
            aria-label="Sign out"
            @click="handleSignout"
          >
            <LogOut />
          </CustomButton>
        </CardAction>
      </div>

      <div class="mt-4">
        <FieldError
          v-if="errorMessage"
          :errors="[{ message: errorMessage }]"
        />

        <p
          v-else-if="isLoading"
          class="text-muted-foreground text-sm"
        >
          Loading users…
        </p>

        <ul
          v-else
          class="flex flex-col gap-2"
        >
          <li
            v-for="user in users"
            :key="user.id"
            class="group relative"
          >
            <div class="bg-muted/50 flex items-center gap-2.5 border px-2.5 py-2">
              <span
                :class="isOnline(user.id) ? 'bg-emerald-500' : 'bg-muted-foreground/30'"
                class="size-2 shrink-0 rounded-full"
              />

              <span>
                {{ user.display_name }}
              </span>

              <span class="text-muted-foreground text-xs">
                {{ `@${user.username}` }}
              </span>

              <Badge
                v-if="user.role === 'admin'"
                class="ml-auto"
              >
                admin
              </Badge>
            </div>

            <CustomCornerMarks reactive />
          </li>
        </ul>
      </div>
    </CustomCard>
  </AppPageContainer>
</template>
