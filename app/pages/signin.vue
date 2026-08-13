<script setup lang="ts">
  import { Field as FormischField, Form as FormischForm, setErrors, useForm } from '@formisch/vue';
  import * as v from 'valibot';

  const loginSchema = v.object({
    usernameOrEmail: v.pipe(v.string(), v.trim(), v.minLength(1, 'Enter your username or email')),
    password: v.pipe(v.string(), v.minLength(1, 'Enter your password')),
  });

  const session = useSession();

  const form = useForm({
    schema: loginSchema,
    initialInput: {
      usernameOrEmail: '',
      password: '',
    },
  });

  async function submit(output: v.InferOutput<typeof loginSchema>): Promise<void> {
    try {
      await session.signin(output);
      await navigateTo('/');
    } catch (error) {
      setErrors(form, {
        errors: [error instanceof Error ? error.message : 'Login failed'],
      });
    }
  }
</script>

<template>
  <AppPageContainer class="flex items-center justify-center p-6">
    <CustomCard class="w-full max-w-sm">
      <CardTitle class="mb-4">Sign in</CardTitle>

      <FormischForm
        :of="form"
        @submit="submit"
      >
        <FieldGroup>
          <FormischField
            v-slot="field"
            :of="form"
            :path="['usernameOrEmail']"
          >
            <Field :data-invalid="field.errors !== null">
              <FieldLabel for="usernameOrEmail">Username or email</FieldLabel>

              <div class="group relative">
                <Input
                  v-model="field.input"
                  v-bind="field.props"
                  :aria-invalid="field.errors !== null"
                  id="usernameOrEmail"
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
            :path="['password']"
          >
            <Field :data-invalid="field.errors !== null">
              <FieldLabel for="password">Password</FieldLabel>

              <div class="group relative">
                <Input
                  v-model="field.input"
                  v-bind="field.props"
                  :aria-invalid="field.errors !== null"
                  id="password"
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
            {{ form.isSubmitting ? 'Signing in…' : 'Sign in' }}
          </CustomButton>

          <CustomLink
            to="/signup"
            aria-label="Sign up"
          >
            Sign up
          </CustomLink>
        </div>
      </FormischForm>
    </CustomCard>
  </AppPageContainer>
</template>
