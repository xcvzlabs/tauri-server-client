<script setup lang="ts">
  import type { AdminUserProfile } from '~/types/api.ts';
  import {
    Field as FormischField,
    Form as FormischForm,
    reset as resetForm,
    setErrors,
    useForm,
  } from '@formisch/vue';
  import {
    ArrowLeft,
    Ban,
    CircleCheck,
    ShieldMinus,
    ShieldPlus,
    Trash2,
    UserPlus,
  } from 'lucide-vue-next';
  import * as v from 'valibot';

  const session = useSession();

  const users = ref<AdminUserProfile[]>([]);
  const isLoading = ref<boolean>(true);
  const errorMessage = ref<string | null>(null);
  const pendingUserId = ref<string | null>(null);
  const isAddUserOpen = ref<boolean>(false);

  async function loadUsers(): Promise<void> {
    isLoading.value = true;
    errorMessage.value = null;
    try {
      users.value = await fetchFromServer<AdminUserProfile[]>('/api/v1/admin/users');
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

  function formatLastActive(user: AdminUserProfile): string {
    if (user.is_online) return 'Online now';
    if (!user.last_seen_at) return 'Never';
    return new Date(user.last_seen_at).toLocaleString();
  }

  async function toggleRole(user: AdminUserProfile): Promise<void> {
    const nextRole = user.role === 'admin' ? 'member' : 'admin';
    errorMessage.value = null;
    pendingUserId.value = user.id;
    try {
      await fetchFromServerVoid(`/api/v1/admin/users/${user.id}/role`, {
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

  async function toggleDisabled(user: AdminUserProfile): Promise<void> {
    errorMessage.value = null;
    pendingUserId.value = user.id;
    try {
      await fetchFromServerVoid(`/api/v1/admin/users/${user.id}/disable`, {
        method: 'PATCH',
        body: JSON.stringify({ is_disabled: !user.is_disabled }),
      });
      await loadUsers();
    } catch (error) {
      errorMessage.value =
        error instanceof Error ? error.message : 'Failed to update account status';
    } finally {
      pendingUserId.value = null;
    }
  }

  async function removeUser(user: AdminUserProfile): Promise<void> {
    if (
      !window.confirm(`Remove ${user.display_name} (@${user.username})? This cannot be undone.`)
    ) {
      return;
    }

    errorMessage.value = null;
    pendingUserId.value = user.id;
    try {
      await fetchFromServerVoid(`/api/v1/admin/users/${user.id}`, { method: 'DELETE' });
      await loadUsers();
    } catch (error) {
      errorMessage.value = error instanceof Error ? error.message : 'Failed to remove user';
    } finally {
      pendingUserId.value = null;
    }
  }

  const addUserSchema = v.object({
    username: v.pipe(
      v.string(),
      v.trim(),
      v.minLength(3, 'Must be at least 3 characters'),
      v.maxLength(32, 'Must be at most 32 characters'),
    ),
    email: v.pipe(v.string(), v.trim(), v.email('Enter a valid email address')),
    displayName: v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter a display name')),
    password: v.pipe(v.string(), v.minLength(8, 'Must be at least 8 characters')),
    role: v.picklist(['member', 'admin']),
  });

  const addUserForm = useForm({
    schema: addUserSchema,
    initialInput: {
      username: '',
      email: '',
      displayName: '',
      password: '',
      role: 'member',
    },
  });

  async function submitAddUser(output: v.InferOutput<typeof addUserSchema>): Promise<void> {
    try {
      await fetchFromServer(`/api/v1/admin/users`, {
        method: 'POST',
        body: JSON.stringify({
          username: output.username,
          email: output.email,
          display_name: output.displayName,
          password: output.password,
          role: output.role,
        }),
      });
      isAddUserOpen.value = false;
      resetForm(addUserForm);
      await loadUsers();
    } catch (error) {
      setErrors(addUserForm, {
        errors: [error instanceof Error ? error.message : 'Failed to add user'],
      });
    }
  }
</script>

<template>
  <AppPageContainer class="flex items-center justify-center p-6">
    <CustomCard class="w-full max-w-2xl">
      <div class="flex">
        <div class="flex-1">
          <CardTitle class="mb-1">Manage users</CardTitle>
          <CardDescription>
            Promote, demote, disable, or remove accounts on this server.
          </CardDescription>
        </div>

        <CardAction class="flex items-center gap-2">
          <Dialog v-model:open="isAddUserOpen">
            <DialogTrigger as-child>
              <CustomButton aria-label="Add user">
                <UserPlus />
              </CustomButton>
            </DialogTrigger>

            <DialogContent>
              <DialogHeader>
                <DialogTitle>Add user</DialogTitle>
                <DialogDescription>
                  Creates an account directly — the person can sign in immediately with this
                  password.
                </DialogDescription>
              </DialogHeader>

              <FormischForm
                :of="addUserForm"
                @submit="submitAddUser"
              >
                <FieldGroup>
                  <FormischField
                    v-slot="field"
                    :of="addUserForm"
                    :path="['username']"
                  >
                    <Field :data-invalid="field.errors !== null">
                      <FieldLabel for="add-user-username">Username</FieldLabel>
                      <div class="group relative">
                        <Input
                          v-model="field.input"
                          v-bind="field.props"
                          :aria-invalid="field.errors !== null"
                          id="add-user-username"
                          type="text"
                          autocomplete="off"
                          autocorrect="off"
                          autocapitalize="off"
                          spellcheck="false"
                          autofocus
                        />

                        <CustomCornerMarks focus-within />
                      </div>
                      <FieldError
                        v-if="field.errors"
                        :errors="field.errors.map((message) => ({ message }))"
                      />
                    </Field>
                  </FormischField>

                  <FormischField
                    v-slot="field"
                    :of="addUserForm"
                    :path="['email']"
                  >
                    <Field :data-invalid="field.errors !== null">
                      <FieldLabel for="add-user-email">Email</FieldLabel>
                      <div class="group relative">
                        <Input
                          v-model="field.input"
                          v-bind="field.props"
                          :aria-invalid="field.errors !== null"
                          id="add-user-email"
                          type="email"
                          autocomplete="off"
                          autocorrect="off"
                          autocapitalize="off"
                          spellcheck="false"
                        />

                        <CustomCornerMarks focus-within />
                      </div>
                      <FieldError
                        v-if="field.errors"
                        :errors="field.errors.map((message) => ({ message }))"
                      />
                    </Field>
                  </FormischField>

                  <FormischField
                    v-slot="field"
                    :of="addUserForm"
                    :path="['displayName']"
                  >
                    <Field :data-invalid="field.errors !== null">
                      <FieldLabel for="add-user-display-name">Display name</FieldLabel>
                      <div class="group relative">
                        <Input
                          v-model="field.input"
                          v-bind="field.props"
                          :aria-invalid="field.errors !== null"
                          id="add-user-display-name"
                          type="text"
                          autocomplete="off"
                          autocorrect="off"
                          autocapitalize="off"
                          spellcheck="false"
                        />

                        <CustomCornerMarks focus-within />
                      </div>
                      <FieldError
                        v-if="field.errors"
                        :errors="field.errors.map((message) => ({ message }))"
                      />
                    </Field>
                  </FormischField>

                  <FormischField
                    v-slot="field"
                    :of="addUserForm"
                    :path="['password']"
                  >
                    <Field :data-invalid="field.errors !== null">
                      <FieldLabel for="add-user-password">Password</FieldLabel>
                      <div class="group relative">
                        <Input
                          v-model="field.input"
                          v-bind="field.props"
                          :aria-invalid="field.errors !== null"
                          id="add-user-password"
                          type="password"
                          autocomplete="off"
                          autocorrect="off"
                          autocapitalize="off"
                          spellcheck="false"
                        />

                        <CustomCornerMarks focus-within />
                      </div>
                      <FieldError
                        v-if="field.errors"
                        :errors="field.errors.map((message) => ({ message }))"
                      />
                    </Field>
                  </FormischField>

                  <FormischField
                    v-slot="field"
                    :of="addUserForm"
                    :path="['role']"
                  >
                    <Field :data-invalid="field.errors !== null">
                      <FieldLabel for="add-user-role">Role</FieldLabel>
                      <div class="group relative">
                        <Select v-model="field.input">
                          <SelectTrigger
                            id="add-user-role"
                            class="w-full"
                          >
                            <SelectValue placeholder="Select a role" />
                          </SelectTrigger>
                          <SelectContent>
                            <SelectItem value="member">Member</SelectItem>
                            <SelectItem value="admin">Admin</SelectItem>
                          </SelectContent>
                        </Select>

                        <CustomCornerMarks focus-within />
                      </div>
                      <FieldError
                        v-if="field.errors"
                        :errors="field.errors.map((message) => ({ message }))"
                      />
                    </Field>
                  </FormischField>

                  <FieldError
                    v-if="addUserForm.errors"
                    :errors="addUserForm.errors.map((message) => ({ message }))"
                  />
                </FieldGroup>

                <DialogFooter class="mt-4">
                  <CustomButton
                    :disabled="addUserForm.isSubmitting"
                    type="submit"
                  >
                    {{ addUserForm.isSubmitting ? 'Adding…' : 'Add user' }}
                  </CustomButton>
                </DialogFooter>
              </FormischForm>
            </DialogContent>
          </Dialog>

          <CustomLink
            to="/home"
            aria-label="Back"
          >
            <ArrowLeft />
          </CustomLink>
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
            <div class="bg-muted/50 flex flex-col gap-1 border px-2.5 py-2">
              <div class="flex items-center gap-2.5">
                <span
                  class="size-2 shrink-0 rounded-full"
                  :class="user.is_online ? 'bg-emerald-500' : 'bg-muted-foreground/30'"
                />
                <span class="text-sm font-medium">{{ user.display_name }}</span>
                <span class="text-muted-foreground text-sm">@{{ user.username }}</span>
                <Badge :variant="user.role === 'admin' ? 'default' : 'secondary'">
                  {{ user.role }}
                </Badge>
                <Badge
                  v-if="user.is_disabled"
                  variant="destructive"
                >
                  disabled
                </Badge>
              </div>

              <p class="text-muted-foreground pl-4.5 text-xs">
                {{ user.email }} · {{ user.ip_address ?? 'no IP recorded' }} · last active:
                {{ formatLastActive(user) }}
              </p>

              <div
                v-if="user.id !== session.currentUser.value?.id"
                class="flex items-center justify-end gap-1.5"
              >
                <CustomButton
                  :disabled="pendingUserId === user.id"
                  :aria-label="user.role === 'admin' ? 'Demote' : 'Promote'"
                  @click="toggleRole(user)"
                >
                  <ShieldMinus v-if="user.role === 'admin'" />
                  <ShieldPlus v-else />
                </CustomButton>

                <CustomButton
                  :disabled="pendingUserId === user.id"
                  :aria-label="user.is_disabled ? 'Enable' : 'Disable'"
                  @click="toggleDisabled(user)"
                >
                  <CircleCheck v-if="user.is_disabled" />
                  <Ban v-else />
                </CustomButton>

                <CustomButton
                  :disabled="pendingUserId === user.id"
                  aria-label="Remove"
                  @click="removeUser(user)"
                >
                  <Trash2 class="text-destructive" />
                </CustomButton>
              </div>
            </div>

            <CustomCornerMarks reactive />
          </li>
        </ul>
      </div>
    </CustomCard>
  </AppPageContainer>
</template>
