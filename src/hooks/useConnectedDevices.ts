import { useSerialStore } from '../stores/serialStore';
import { useBleStore } from '../stores/bleStore';

export interface ConnectedDevice {
  id: string;
  name: string;
  type: 'serial' | 'ble';
}

export const useConnectedDevices = (): ConnectedDevice[] => {
  const tabs = useSerialStore((state) => state.tabs);
  const connections = useBleStore((state) => state.connections);

  const serialDevices: ConnectedDevice[] = tabs
    .filter((tab) => tab.tabType === 'port' && tab.isConnected)
    .map((tab) => ({
      id: tab.portName,
      name: tab.portName,
      type: 'serial' as const,
    }));

  const bleDevices: ConnectedDevice[] = connections.map((conn) => ({
    id: conn.address,
    name: conn.name || conn.address,
    type: 'ble' as const,
  }));

  return [...serialDevices, ...bleDevices];
};

export const getConnectedDevices = (): ConnectedDevice[] => {
  const { tabs } = useSerialStore.getState();
  const { connections } = useBleStore.getState();

  const serialDevices: ConnectedDevice[] = tabs
    .filter((tab) => tab.tabType === 'port' && tab.isConnected)
    .map((tab) => ({
      id: tab.portName,
      name: tab.portName,
      type: 'serial' as const,
    }));

  const bleDevices: ConnectedDevice[] = connections.map((conn) => ({
    id: conn.address,
    name: conn.name || conn.address,
    type: 'ble' as const,
  }));

  return [...serialDevices, ...bleDevices];
};
