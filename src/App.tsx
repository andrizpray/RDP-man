import { useEffect } from 'react';
import { useStore, type ConnectionProfile } from './store';
import { ConnectionForm } from './components/ConnectionForm';
import { HistoryView } from './components/HistoryView';
import { RdpViewer } from './components/RdpViewer';
import './App.css';

function App() {
  const {
    connections, healthMap, activeSessions, currentView, showAddModal, editingConnection,
    loadConnections, loadSessionLogs, removeConnection, refreshHealth, refreshSessions,
    openRdp, closeRdp, drainEndedSessions, setView, setShowAddModal, setEditing,
  } = useStore();

  useEffect(() => {
    loadConnections();
    refreshHealth();
    refreshSessions();
    const id = setInterval(() => { refreshHealth(); refreshSessions(); drainEndedSessions(); }, 10_000);
    return () => clearInterval(id);
  }, []);

  const handleViewHistory = () => { setView('history'); loadSessionLogs(); };
  const isActive = (cid: number) => activeSessions.some((s) => s.connection_id === cid);
  const getSid = (cid: number) => activeSessions.find((s) => s.connection_id === cid)?.session_id;

  return (
    <div className="app">
      <header className="header">
        <h1 onClick={() => setView('dashboard')}>🖥️ RDP Man</h1>
        <nav>
          <button className={currentView === 'dashboard' ? 'active' : ''} onClick={() => setView('dashboard')}>
            Dashboard{activeSessions.length > 0 && <span className="badge-count">{activeSessions.length}</span>}
          </button>
          <button className={currentView === 'history' ? 'active' : ''} onClick={handleViewHistory}>History</button>
        </nav>
      </header>

       {currentView === 'dashboard' && (
         <>
           {activeSessions.length === 0 ? (
             <main className="dashboard">
               <div className="toolbar">
                 <h2>Servers ({connections.length})</h2>
                 <div className="toolbar-actions">
                   <button className="btn-secondary" onClick={refreshHealth}>↻ Refresh</button>
                   <button className="btn-primary" onClick={() => { setEditing(null); setShowAddModal(true); }}>+ Add Server</button>
                 </div>
               </div>

               {connections.length === 0 ? (
                 <div className="empty">No servers yet. Click "+ Add Server" to get started.</div>
               ) : (
                 <div className="server-grid">
                   {connections.map((conn) => (
                     <ServerCard
                       key={conn.id}
                       conn={conn}
                       status={healthMap[conn.id] ?? 'checking'}
                       active={isActive(conn.id)}
                       onConnect={() => openRdp(conn.id)}
                       onDisconnect={() => { const sid = getSid(conn.id); if (sid !== undefined) closeRdp(sid); }}
                       onEdit={() => { setEditing(conn); setShowAddModal(true); }}
                       onDelete={() => removeConnection(conn.id)}
                     />
                   ))}
                 </div>
               )}
             </main>
           ) : (
             <div className="rdp-viewer-container">
               {activeSessions.map((session) => (
                 <RdpViewer
                   key={session.session_id}
                   sessionId={session.session_id}
                   width={session.width}
                   height={session.height}
                   onClose={() => closeRdp(session.session_id)}
                 />
               ))}
             </div>
           )}
         </>
       )}
       {currentView === 'history' && <HistoryView />}

      {showAddModal && (
        <ConnectionForm editing={editingConnection} onClose={() => { setShowAddModal(false); setEditing(null); }} />
      )}
    </div>
  );
}

function ServerCard({ conn, status, active, onConnect, onDisconnect, onEdit, onDelete }: {
  conn: ConnectionProfile;
  status: 'online' | 'offline' | 'checking';
  active: boolean;
  onConnect: () => void;
  onDisconnect: () => void;
  onEdit: () => void;
  onDelete: () => void;
}) {
  const dot = status === 'online' ? 'status-online' : status === 'offline' ? 'status-offline' : 'status-checking';

  return (
    <div className={`server-card ${active ? 'server-card-active' : ''}`}>
      <div className="server-card-header">
        <span className={`status-dot ${dot}`} />
        <span className="server-name">{conn.display_name}</span>
        {active && <span className="badge-live">LIVE</span>}
      </div>
      <div className="server-card-body">
        <div className="server-info">{conn.hostname}:{conn.port}</div>
        <div className="server-info">{conn.username}</div>
      </div>
      <div className="server-card-actions">
        {active ? (
          <button className="btn-disconnect" onClick={onDisconnect}>Disconnect</button>
        ) : (
          <button className="btn-connect" onClick={onConnect} disabled={status === 'offline'}>
            Connect
          </button>
        )}
        <button className="btn-icon" onClick={onEdit} title="Edit">✏️</button>
        <button className="btn-icon" onClick={onDelete} title="Delete">🗑️</button>
      </div>
    </div>
  );
}

export default App;
