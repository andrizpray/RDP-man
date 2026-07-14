import { useState } from 'react';
import { useStore, type ConnectionProfile } from '../store';

interface Props {
  editing: ConnectionProfile | null;
  onClose: () => void;
}

export function ConnectionForm({ editing, onClose }: Props) {
  const { addConnection, updateConnection } = useStore();
  const [form, setForm] = useState({
    display_name: editing?.display_name ?? '',
    hostname: editing?.hostname ?? '',
    port: editing?.port ?? 3389,
    username: editing?.username ?? '',
    password: editing?.password ?? '',
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (editing) {
      await updateConnection({ ...form, id: editing.id });
    } else {
      await addConnection(form);
    }
    onClose();
  };

  const set = (key: string, value: string | number) =>
    setForm((prev) => ({ ...prev, [key]: value }));

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <h2>{editing ? 'Edit Server' : 'Add Server'}</h2>
        <form onSubmit={handleSubmit}>
          <label>
            Display Name
            <input value={form.display_name} onChange={(e) => set('display_name', e.target.value)} required />
          </label>
          <label>
            Hostname / IP
            <input value={form.hostname} onChange={(e) => set('hostname', e.target.value)} required />
          </label>
          <label>
            Port
            <input type="number" value={form.port} onChange={(e) => set('port', parseInt(e.target.value) || 3389)} />
          </label>
          <label>
            Username
            <input value={form.username} onChange={(e) => set('username', e.target.value)} required />
          </label>
          <label>
            Password
            <input type="password" value={form.password} onChange={(e) => set('password', e.target.value)} required />
          </label>
          <div className="modal-actions">
            <button type="button" className="btn-secondary" onClick={onClose}>Cancel</button>
            <button type="submit" className="btn-primary">{editing ? 'Save' : 'Add'}</button>
          </div>
        </form>
      </div>
    </div>
  );
}
