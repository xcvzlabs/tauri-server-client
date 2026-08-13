type SessionStatus = 'idle' | 'restoring' | 'authenticated' | 'unauthenticated';

type SigninPayload = {
  usernameOrEmail: string;
  password: string;
};

type SignupPayload = {
  username: string;
  email: string;
  password: string;
  displayName: string;
};

const SESSION_STORE_FILE = 'session.json';
const SERVER_URL_KEY = 'serverUrl';
const REFRESH_TOKEN_KEY = 'refreshToken';

const serverURL = ref<NormalizedServerURL | null>(null);
const accessToken = ref<string | null>(null);
const currentUser = ref<MeProfile | null>(null);
const status = ref<SessionStatus>('idle');

let storePromise: ReturnType<typeof useTauriStoreLoad> | null = null;
let restorePromise: Promise<void> | null = null;

// Singleton: an account can be disabled (or its refresh token revoked) while
// the user is already signed in — force them back to /signin the instant
// that happens instead of leaving them stuck on a protected page with a
// session the server no longer honors.
watch(status, (current, previous) => {
  if (previous === 'authenticated' && current === 'unauthenticated') {
    void navigateTo('/signin');
  }
});

export function useSession() {
  function getStore() {
    storePromise ??= useTauriStoreLoad(SESSION_STORE_FILE);
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
      const tokens = await fetchFromServer<TokenResponse>('/api/v1/auth/refresh', {
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
        const persistedServerURL = await store.get<string>(SERVER_URL_KEY);

        if (!persistedServerURL) {
          status.value = 'unauthenticated';
          return;
        }

        serverURL.value = normalizeServerURL(persistedServerURL);

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

  async function setServerURL(input: string): Promise<void> {
    const normalized = normalizeServerURL(input);
    const response = await fetch(`${normalized.httpBase}/health`).catch(() => null);

    if (!response || !response.ok) {
      throw new Error('Could not reach a server at that address');
    }

    serverURL.value = normalized;

    const store = await getStore();
    await store.set(SERVER_URL_KEY, normalized.httpBase);
    await store.save();

    status.value = 'unauthenticated';
  }

  async function signin(payload: SigninPayload): Promise<void> {
    const tokens = await fetchFromServer<TokenResponse>('/api/v1/auth/signin', {
      method: 'POST',
      body: JSON.stringify({
        username_or_email: payload.usernameOrEmail,
        password: payload.password,
      }),
    });

    await applyTokenResponse(tokens);
  }

  async function signup(payload: SignupPayload): Promise<void> {
    const tokens = await fetchFromServer<TokenResponse>('/api/v1/auth/signup', {
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

  async function signout(): Promise<void> {
    const store = await getStore();
    const persistedRefreshToken = await store.get<string>(REFRESH_TOKEN_KEY);

    if (persistedRefreshToken) {
      await fetchFromServerVoid('/api/v1/auth/signout', {
        method: 'POST',
        body: JSON.stringify({ refresh_token: persistedRefreshToken }),
      }).catch((error) => {
        console.error(error);
      });
    }

    await clearSession();
  }

  return {
    serverURL: readonly(serverURL),
    accessToken: readonly(accessToken),
    currentUser: readonly(currentUser),
    status: readonly(status),
    isAuthenticated: computed(() => status.value === 'authenticated'),
    restoreSession,
    setServerURL,
    signin,
    signup,
    signout,
    refreshTokens,
    clearSession,
  };
}
