import { useEffect } from 'react';
import { useStore } from '../store';

export function HistoryView() {
  const { sessionLogs, loadSessionLogs } = useStore();

  useEffect(() => { loadSessionLogs(); }, []);

  const formatDuration = (sec: number | null) => {
    if (sec === null) return '-';
    const m = Math.floor(sec / 60);
    const s = sec % 60;
    return m > 0 ? `${m}m ${s}s` : `${s}s`;
  };

  return (
    <main className="history">
      <h2>Session History</h2>
      {sessionLogs.length === 0 ? (
        <div className="empty">No sessions recorded yet.</div>
      ) : (
        <table className="history-table">
          <thead>
            <tr>
              <th>Date</th>
              <th>Server</th>
              <th>Username</th>
              <th>Duration</th>
              <th>Status</th>
            </tr>
          </thead>
          <tbody>
            {sessionLogs.map((log) => (
              <tr key={log.id}>
                <td>{new Date(log.connected_at).toLocaleString()}</td>
                <td>{log.hostname}</td>
                <td>{log.username}</td>
                <td>{formatDuration(log.duration_sec)}</td>
                <td>
                  <span className={`badge badge-${log.status}`}>{log.status}</span>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </main>
  );
}
