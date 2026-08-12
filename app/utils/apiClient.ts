import type { ApiErrorBody } from '~/types/api.ts';

function createApiError(code: string, message: string): Error {
  return Object.assign(new Error(message), { code });
}

async function parseErrorBody(response: Response): Promise<ApiErrorBody['error'] | null> {
  const body: unknown = await response.json().catch(() => null);

  if (
    typeof body === 'object' &&
    body !== null &&
    'error' in body &&
    typeof body.error === 'object' &&
    body.error !== null &&
    'code' in body.error &&
    'message' in body.error &&
    typeof body.error.code === 'string' &&
    typeof body.error.message === 'string'
  ) {
    return { code: body.error.code, message: body.error.message };
  }

  return null;
}

/**
 * Sends a request to the LAN server: resolves the base URL and bearer token from
 * `useSession()`, and on a 401 from a non-auth endpoint attempts one silent token
 * refresh before retrying. Auth endpoints themselves are excluded from that retry so a
 * failed login/refresh can't recurse into another refresh attempt. Throws on any
 * non-2xx response.
 */
async function sendRequest(path: string, init: RequestInit, retrying: boolean): Promise<Response> {
  const session = useSession();
  if (!session.serverUrl.value) {
    throw createApiError('NOT_CONNECTED', 'Not connected to a server');
  }

  const headers = new Headers(init.headers);
  headers.set('content-type', 'application/json');
  if (session.accessToken.value) {
    headers.set('authorization', `Bearer ${session.accessToken.value}`);
  }

  const response = await fetch(`${session.serverUrl.value.httpBase}${path}`, { ...init, headers });

  const isAuthEndpoint = path.startsWith('/api/v1/auth/');
  if (response.status === 401 && !retrying && !isAuthEndpoint && session.accessToken.value) {
    const refreshed = await session.refreshTokens();
    if (refreshed) {
      return sendRequest(path, init, true);
    }
  }

  if (!response.ok) {
    const errorBody = await parseErrorBody(response);
    throw createApiError(
      errorBody?.code ?? 'UNKNOWN_ERROR',
      errorBody?.message ?? `Request failed with status ${response.status}`,
    );
  }

  return response;
}

export async function apiFetch<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await sendRequest(path, init, false);
  return await response.json();
}

export async function apiSend(path: string, init: RequestInit = {}): Promise<void> {
  await sendRequest(path, init, false);
}
