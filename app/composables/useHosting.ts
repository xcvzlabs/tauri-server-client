type HostInfo = {
  host: string;
  port: number;
};

const isHosting = ref<boolean>(false);
const hostInfo = ref<HostInfo | null>(null);

export function useHosting() {
  async function start(): Promise<HostInfo> {
    const info = await useTauriCoreInvoke<HostInfo>('start_server');
    isHosting.value = true;
    hostInfo.value = info;
    return info;
  }

  async function stop(): Promise<void> {
    await useTauriCoreInvoke('stop_server');
    isHosting.value = false;
    hostInfo.value = null;
  }

  return {
    isHosting: readonly(isHosting),
    hostInfo: readonly(hostInfo),
    start,
    stop,
  };
}
