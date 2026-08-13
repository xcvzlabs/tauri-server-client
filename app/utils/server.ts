const AUTH_RESOURCE_PATH = '/api/v1/auth/';

function HTTPError(error: ApiErrorBody['error']): Error {
  return Object.assign(new Error(error.message, { cause: error }), { code: error.code });
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
    return {
      code: body.error.code,
      message: body.error.message,
    };
  }

  return null;
}

async function request(path: string, init: RequestInit, retrying: boolean): Promise<Response> {
  const session = useSession();

  if (!session.serverURL.value) {
    throw HTTPError({
      code: 'NOT_CONNECTED',
      message: 'Not connected to a server',
    });
  }

  const headers = new Headers(init.headers);
  headers.set('content-type', 'application/json');

  if (session.accessToken.value) {
    headers.set('authorization', `Bearer ${session.accessToken.value}`);
  }

  const url = `${session.serverURL.value.httpBase}${path}`;
  const response = await useTauriHTTPFetch(url, { ...init, headers });
  const isAuthEndpoint = path.startsWith(AUTH_RESOURCE_PATH);

  if (response.status === 401 && !retrying && !isAuthEndpoint && session.accessToken.value) {
    const refreshed = await session.refreshTokens();
    if (refreshed) return request(path, init, true);
  }

  if (!response.ok) {
    const error = await parseErrorBody(response);

    if (error?.code === 'ACCOUNT_DISABLED') {
      await session.clearSession();
    }

    throw HTTPError({
      code: error?.code ?? 'UNKNOWN_ERROR',
      message: error?.message ?? `Request failed with status ${response.status}`,
    });
  }

  return response;
}

export async function fetchFromServer<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await request(path, init, false);
  return await response.json();
}

export async function fetchFromServerVoid(path: string, init: RequestInit = {}): Promise<void> {
  await request(path, init, false);
}
