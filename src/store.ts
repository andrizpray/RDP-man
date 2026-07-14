import { create } from 'zustand';
import { invoke } from '@tauri-apps/api/core';

export interface ConnectionProfile {
  id: number;
  display_name: string;
  hostname: string;
  port: number;
  username: string;
  password: string;
}

export interface SessionLog {
  id: number;
  connection_id: number;
  hostname: string;
  username: string;
  connected_at: string;
  disconnected_at: string | null;
  duration_sec: number | null;
  status: string;
}

export interface HealthResult {
  id: number;
  hostname: string;
  port: number;
  status: 'online' | 'offline';
}

export interface SessionInfo {
  session_id: number;
  connection_id: number;
  hostname: string;
  pid: number;
  status: string;
}

type View = 'dashboard' | 'history';

interface Store {
  connections: ConnectionProfile[];
  healthMap: Record<number, 'online' | 'offline' | 'checking'>;
  activeSessions: SessionInfo[];
  sessionLogs: SessionLog[];
  currentView: View;
  editingConnection: ConnectionProfile | null;
  showAddModal: boolean;

  loadConnections: () => Promise<void>;
  addConnection: (data: Omit<ConnectionProfile, 'id'>) => Promise<void>;
  updateConnection: (data: ConnectionProfile) => Promise<void>;
  removeConnection: (id: number) => Promise<void>;
  refreshHealth: () => Promise<void>;
  openRdp: (connectionId: number) => Promise<void>;
  closeRdp: (sessionId: number) => Promise<void>;
  refreshSessions: () => Promise<void>;
  loadSessionLogs: () => Promise<void>;
  setView: (view: View) => void;
  setEditing: (conn: ConnectionProfile | null) => void;
  setShowAddModal: (show: boolean) => void;
}

export const useStore = create<Store>((set, get) => ({
  connections: [],
  healthMap: {},
  activeSessions: [],
  sessionLogs: [],
  currentView: 'dashboard',
  editingConnection: null,
  showAddModal: false,

  loadConnections: async () => {
    const conns = await invoke<ConnectionProfile[]>('get_connections');
    set({ connections: conns });
  },

  addConnection: async (data) => {
    await invoke('add_connection', {
      name: data.display_name,
      host: data.hostname,
      port: data.port,
      user: data.username,
      pass: data.password,
    });
    await get().loadConnections();
    get().refreshHealth();
  },

  updateConnection: async (data) => {
    await invoke('update_connection', {
      id: data.id,
      name: data.display_name,
      host: data.hostname,
      port: data.port,
      user: data.username,
      pass: data.password,
    });
    await get().loadConnections();
    get().refreshHealth();
  },

  removeConnection: async (id) => {
    await invoke('remove_connection', { id });
    await get().loadConnections();
  },

  refreshHealth: async () => {
    const results = await invoke<HealthResult[]>('check_all_servers');
    const map: Record<number, 'online' | 'offline'> = {};
    for (const r of results) map[r.id] = r.status;
    set({ healthMap: map });
  },

  openRdp: async (connectionId) => {
    try {
      await invoke('open_rdp_session', { connectionId });
      get().refreshSessions();
    } catch (e) {
      alert(`RDP Error: ${e}`);
    }
  },

  closeRdp: async (sessionId) => {
    await invoke('close_rdp_session', { sessionId });
    get().refreshSessions();
  },

  refreshSessions: async () => {
    const sessions = await invoke<SessionInfo[]>('get_active_sessions');
    set({ activeSessions: sessions });
  },

  loadSessionLogs: async () => {
    const logs = await invoke<SessionLog[]>('get_session_logs');
    set({ sessionLogs: logs });
  },

  setView: (view) => set({ currentView: view }),
  setEditing: (conn) => set({ editingConnection: conn }),
  setShowAddModal: (show) => set({ showAddModal: show }),
}));
