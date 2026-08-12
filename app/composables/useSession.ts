import type { MeProfile, TokenResponse } from '~/types/api.ts';
import { Store } from '@tauri-apps/plugin-store';
import { normalizeServerUrl, type NormalizedServerUrl } from '~/utils/serverUrl.ts';

type SessionStatus = 'idle' | 'restoring' | 'authenticated' | 'unauthenticated';

type LoginPayload = {
  usernameOrEmail: string;
  password: string;
};

type RegisterPayload = {
  username: string;
  email: string;
  password: string;
  displayName: string;
};

const SESSION_STORE_FILE = 'session.json';
const SERVER_URL_KEY = 'serverUrl';
const REFRESH_TOKEN_KEY = 'refreshToken';

const serverUrl = ref<NormalizedServerUrl | null>(null);
const accessToken = ref<string | null>(null);
const currentUser = ref<MeProfile | null>(null);
const status = ref<SessionStatus>('idle');

let storePromise: ReturnType<typeof Store.load> | null = null;
let restorePromise: Promise<void> | null = null;

function getStore() {
  storePromise ??= Store.load(SESSION_STORE_FILE);
  return storePromise;
}

async function persistRefreshToken(refreshToken: string | null): Promise<void> {
  const store = await getStore();
  if (refreshToken) await store.set(REFRESH_TOKEN_KEY, refreshToken);
  else await store.delete(REFRESH_TOKEN_KEY);
  await store.save();
}

function applyTokenResponse(tokens: TokenResponse): Promise<void> {
  accessToken.value = tokens.access_token;
  currentUser.value = tokens.user;
  status.value = 'authenticated';
  return persistRefreshToken(tokens.refresh_token);
}

function clearSession(): Promise<void> {
  accessToken.value = null;
  currentUser.value = null;
  status.value = 'unauthenticated';
  return persistRefreshToken(null);
}

async function refreshWithToken(refreshToken: string): Promise<boolean> {
  try {
    const tokens = await apiFetch<TokenResponse>('/api/v1/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
    await applyTokenResponse(tokens);
    return true;
  } catch {
    await clearSession();
    return false;
  }
}

async function refreshTokens(): Promise<boolean> {
  const store = await getStore();
  const persistedRefreshToken = await store.get<string>(REFRESH_TOKEN_KEY);
  return !persistedRefreshToken ? false : refreshWithToken(persistedRefreshToken);
}

function restoreSession(): Promise<void> {
  restorePromise ??= (async () => {
    status.value = 'restoring';

    try {
      const store = await getStore();
      const persistedServerUrl = await store.get<string>(SERVER_URL_KEY);

      if (!persistedServerUrl) {
        status.value = 'unauthenticated';
        return;
      }

      serverUrl.value = normalizeServerUrl(persistedServerUrl);

      const persistedRefreshToken = await store.get<string>(REFRESH_TOKEN_KEY);

      if (!persistedRefreshToken) {
        status.value = 'unauthenticated';
        return;
      }

      const restored = await refreshWithToken(persistedRefreshToken);
      status.value = restored ? 'authenticated' : 'unauthenticated';
    } catch (error) {
      console.error('Failed to restore session', error);
      status.value = 'unauthenticated';
    }
  })();

  return restorePromise;
}

async function setServerUrl(input: string): Promise<void> {
  const normalized = normalizeServerUrl(input);
  const response = await fetch(`${normalized.httpBase}/health`).catch(() => null);

  if (!response || !response.ok) {
    throw new Error('Could not reach a server at that address');
  }

  serverUrl.value = normalized;

  const store = await getStore();
  await store.set(SERVER_URL_KEY, normalized.httpBase);
  await store.save();

  status.value = 'unauthenticated';
}

async function login(payload: LoginPayload): Promise<void> {
  const tokens = await apiFetch<TokenResponse>('/api/v1/auth/login', {
    method: 'POST',
    body: JSON.stringify({
      username_or_email: payload.usernameOrEmail,
      password: payload.password,
    }),
  });

  await applyTokenResponse(tokens);
}

async function register(payload: RegisterPayload): Promise<void> {
  const tokens = await apiFetch<TokenResponse>('/api/v1/auth/register', {
    method: 'POST',
    body: JSON.stringify({
      username: payload.username,
      email: payload.email,
      password: payload.password,
      display_name: payload.displayName,
    }),
  });

  await applyTokenResponse(tokens);
}

async function logout(): Promise<void> {
  const store = await getStore();
  const persistedRefreshToken = await store.get<string>(REFRESH_TOKEN_KEY);

  if (persistedRefreshToken) {
    await apiSend('/api/v1/auth/logout', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: persistedRefreshToken }),
    }).catch(() => {
      //
    });
  }

  await clearSession();
}

export function useSession() {
  return {
    serverUrl: readonly(serverUrl),
    accessToken: readonly(accessToken),
    currentUser: readonly(currentUser),
    status: readonly(status),
    isAuthenticated: computed(() => status.value === 'authenticated'),
    restoreSession,
    setServerUrl,
    login,
    register,
    logout,
    refreshTokens,
  };
}
