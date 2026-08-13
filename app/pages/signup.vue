<script setup lang="ts">
  import { Field as FormischField, Form as FormischForm, setErrors, useForm } from '@formisch/vue';
  import * as v from 'valibot';

  const signupSchema = v.object({
    username: v.pipe(
      v.string(),
      v.trim(),
      v.minLength(3, 'Must be at least 3 characters'),
      v.maxLength(32, 'Must be at most 32 characters'),
    ),
    email: v.pipe(v.string(), v.trim(), v.email('Enter a valid email address')),
    displayName: v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter a display name')),
    password: v.pipe(v.string(), v.minLength(8, 'Must be at least 8 characters')),
  });

  const session = useSession();

  const form = useForm({
    schema: signupSchema,
    initialInput: {
      username: '',
      email: '',
      displayName: '',
      password: '',
    },
  });

  async function submit(output: v.InferOutput<typeof signupSchema>): Promise<void> {
    try {
      await session.signup(output);
      await navigateTo('/');
    } catch (error) {
      setErrors(form, {
        errors: [error instanceof Error ? error.message : 'Registration failed'],
      });
    }
  }
</script>

<template>
  <AppPageContainer class="flex items-center justify-center p-6">
    <CustomCard class="w-full max-w-sm">
      <CardTitle class="mb-4">Sign up</CardTitle>

      <FormischForm
        :of="form"
        @submit="submit"
      >
        <FieldGroup>
          <FormischField
            v-slot="field"
            :of="form"
            :path="['username']"
          >
            <Field :data-invalid="field.errors !== null">
              <FieldLabel for="username">Username</FieldLabel>

              <div class="group relative">
                <Input
                  id="username"
                  v-model="field.input"
                  v-bind="field.props"
                  :aria-invalid="field.errors !== null"
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
            :of="form"
            :path="['email']"
          >
            <Field :data-invalid="field.errors !== null">
              <FieldLabel for="email">Email</FieldLabel>

              <div class="group relative">
                <Input
                  id="email"
                  v-model="field.input"
                  v-bind="field.props"
                  :aria-invalid="field.errors !== null"
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
            :of="form"
            :path="['displayName']"
          >
            <Field :data-invalid="field.errors !== null">
              <FieldLabel for="displayName">Display name</FieldLabel>

              <div class="group relative">
                <Input
                  id="displayName"
                  v-model="field.input"
                  v-bind="field.props"
                  :aria-invalid="field.errors !== null"
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
            :of="form"
            :path="['password']"
          >
            <Field :data-invalid="field.errors !== null">
              <FieldLabel for="password">Password</FieldLabel>

              <div class="group relative">
                <Input
                  id="password"
                  v-model="field.input"
                  v-bind="field.props"
                  :aria-invalid="field.errors !== null"
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

          <FieldError
            v-if="form.errors"
            :errors="form.errors.map((message) => ({ message }))"
          />
        </FieldGroup>

        <div class="mt-4 flex items-center justify-end gap-2">
          <CustomButton
            :disabled="form.isSubmitting"
            type="submit"
          >
            {{ form.isSubmitting ? 'Creating account…' : 'Sign up' }}
          </CustomButton>

          <CustomLink
            to="/signin"
            aria-label="Sign in"
          >
            Sign in
          </CustomLink>
        </div>
      </FormischForm>
    </CustomCard>
  </AppPageContainer>
</template>
