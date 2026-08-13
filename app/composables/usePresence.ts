import * as v from 'valibot';

type ConnectionState = 'disconnected' | 'connecting' | 'connected';

// Capped exponential backoff; the last delay repeats indefinitely so the client keeps
// retrying a server that restarts or a LAN link that comes back after a while.
const RECONNECT_DELAYS_MS = [1000, 2000, 5000, 10000, 15000];

const onlineUserIds = ref<Set<string>>(new Set());
const connectionState = ref<ConnectionState>('disconnected');

let socket: WebSocket | null = null;
let reconnectAttempt = 0;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let closedIntentionally = true;

function clearReconnectTimer(): void {
  if (reconnectTimer) {
    clearTimeout(reconnectTimer);
    reconnectTimer = null;
  }
}

function handleMessage(event: MessageEvent<string>): void {
  let payload: v.InferOutput<typeof presenceEventSchema>;
  try {
    payload = v.parse(presenceEventSchema, JSON.parse(event.data));
  } catch {
    return;
  }

  if (payload.type === 'snapshot') {
    onlineUserIds.value = new Set(payload.online_user_ids);
    return;
  }

  const next = new Set(onlineUserIds.value);
  if (payload.type === 'user_online') {
    next.add(payload.user_id);
  } else {
    next.delete(payload.user_id);
  }
  onlineUserIds.value = next;
}

async function scheduleReconnect(): Promise<void> {
  if (closedIntentionally) {
    return;
  }

  // A dropped socket may just mean the short-lived access token expired; refresh
  // before retrying since a WebSocket upgrade rejection carries no readable status.
  const session = useSession();
  await session.refreshTokens().catch(() => null);

  const delay = RECONNECT_DELAYS_MS[Math.min(reconnectAttempt, RECONNECT_DELAYS_MS.length - 1)];
  reconnectAttempt += 1;
  clearReconnectTimer();
  reconnectTimer = setTimeout(() => {
    openSocket();
  }, delay);
}

function openSocket(): void {
  const session = useSession();
  if (closedIntentionally || !session.serverURL.value || !session.accessToken.value) {
    return;
  }

  connectionState.value = 'connecting';
  const url = `${session.serverURL.value.wsBase}/ws/presence?token=${encodeURIComponent(session.accessToken.value)}`;
  const ws = new WebSocket(url);
  socket = ws;

  ws.addEventListener('open', () => {
    connectionState.value = 'connected';
    reconnectAttempt = 0;
  });
  ws.addEventListener('message', handleMessage);
  ws.addEventListener('close', () => {
    connectionState.value = 'disconnected';
    onlineUserIds.value = new Set();
    if (socket === ws) {
      socket = null;
      void scheduleReconnect();
    }
  });
  ws.addEventListener('error', () => {
    ws.close();
  });
}

function connect(): void {
  closedIntentionally = false;
  reconnectAttempt = 0;
  openSocket();
}

function disconnect(): void {
  closedIntentionally = true;
  clearReconnectTimer();
  onlineUserIds.value = new Set();
  connectionState.value = 'disconnected';
  socket?.close();
  socket = null;
}

// Singleton: follow session auth state directly rather than making every page wire up
// connect/disconnect calls itself.
watch(
  useSession().isAuthenticated,
  (authenticated) => {
    if (authenticated) {
      connect();
    } else {
      disconnect();
    }
  },
  { immediate: true },
);

export function usePresence() {
  return {
    onlineUserIds: readonly(onlineUserIds),
    connectionState: readonly(connectionState),
  };
}
