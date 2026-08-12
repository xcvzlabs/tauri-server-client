import { invoke } from '@tauri-apps/api/core';

type HostInfo = {
  host: string;
  port: number;
};

const isHosting = ref(false);
const hostInfo = ref<HostInfo | null>(null);

async function start(): Promise<HostInfo> {
  const info = await invoke<HostInfo>('start_server');
  isHosting.value = true;
  hostInfo.value = info;
  return info;
}

async function stop(): Promise<void> {
  await invoke('stop_server');
  isHosting.value = false;
  hostInfo.value = null;
}

export function useHosting() {
  return {
    isHosting: readonly(isHosting),
    hostInfo: readonly(hostInfo),
    start,
    stop,
  };
}
