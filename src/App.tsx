import { useEffect, useState } from 'react';
import { useStore, type ConnectionProfile } from './store';
import { ConnectionForm } from './components/ConnectionForm';
import { HistoryView } from './components/HistoryView';
import './App.css';

function App() {
  const {
    connections, currentView, showAddModal, editingConnection,
    loadConnections, loadSessionLogs, removeConnection,
    setView, setShowAddModal, setEditing,
  } = useStore();

  useEffect(() => { loadConnections(); }, []);

  const handleViewHistory = () => {
    setView('history');
    loadSessionLogs();
  };

  return (
    <div className="app">
      <header className="header">
        <h1 onClick={() => setView('dashboard')}>🖥️ RDP Man</h1>
        <nav>
          <button className={currentView === 'dashboard' ? 'active' : ''} onClick={() => setView('dashboard')}>Dashboard</button>
          <button className={currentView === 'history' ? 'active' : ''} onClick={handleViewHistory}>History</button>
        </nav>
      </header>

      {currentView === 'dashboard' ? (
        <main className="dashboard">
          <div className="toolbar">
            <h2>Servers ({connections.length})</h2>
            <button className="btn-primary" onClick={() => { setEditing(null); setShowAddModal(true); }}>+ Add Server</button>
          </div>

          {connections.length === 0 ? (
            <div className="empty">No servers yet. Click "+ Add Server" to get started.</div>
          ) : (
            <div className="server-grid">
              {connections.map((conn) => (
                <ServerCard
                  key={conn.id}
                  conn={conn}
                  onEdit={() => { setEditing(conn); setShowAddModal(true); }}
                  onDelete={() => removeConnection(conn.id)}
                />
              ))}
            </div>
          )}
        </main>
      ) : (
        <HistoryView />
      )}

      {showAddModal && (
        <ConnectionForm
          editing={editingConnection}
          onClose={() => { setShowAddModal(false); setEditing(null); }}
        />
      )}
    </div>
  );
}

function ServerCard({ conn, onEdit, onDelete }: {
  conn: ConnectionProfile;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const [status, setStatus] = useState<'online' | 'offline' | 'checking'>('checking');

  useEffect(() => {
    // TCP probe via Tauri command (stub for now — Phase 3)
    setStatus('checking');
    const timeout = setTimeout(() => setStatus('offline'), 2000);
    return () => clearTimeout(timeout);
  }, [conn.hostname, conn.port]);

  const statusClass = status === 'online' ? 'status-online' : status === 'offline' ? 'status-offline' : 'status-checking';

  return (
    <div className="server-card">
      <div className="server-card-header">
        <span className={`status-dot ${statusClass}`} />
        <span className="server-name">{conn.display_name}</span>
      </div>
      <div className="server-card-body">
        <div className="server-info">{conn.hostname}:{conn.port}</div>
        <div className="server-info">{conn.username}</div>
      </div>
      <div className="server-card-actions">
        <button className="btn-connect" onClick={() => {}}>Connect</button>
        <button className="btn-icon" onClick={onEdit} title="Edit">✏️</button>
        <button className="btn-icon" onClick={onDelete} title="Delete">🗑️</button>
      </div>
    </div>
  );
}

export default App;
