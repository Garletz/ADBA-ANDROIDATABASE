import { useEffect, useState, useCallback } from 'react';
import { getStatus, getDatabases, getConnectionInfo, regeneratePairingCode, createDatabase } from './api';
import type { ServerStatus, DatabaseInfo, ConnectionInfo } from './api';
import './App.css';

function App() {
  const [status, setStatus] = useState<ServerStatus | null>(null);
  const [databases, setDatabases] = useState<DatabaseInfo[]>([]);
  const [connectionInfo, setConnectionInfo] = useState<ConnectionInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [showAddDb, setShowAddDb] = useState(false);
  const [newDbName, setNewDbName] = useState('');
  const [newDbApp, setNewDbApp] = useState('');

  const fetchData = useCallback(async () => {
    try {
      const [statusData, dbsData, connData] = await Promise.all([
        getStatus(),
        getDatabases(),
        getConnectionInfo()
      ]);
      setStatus(statusData);
      setDatabases(dbsData);
      setConnectionInfo(connData);
    } catch (err) {
      console.error('Failed to fetch data:', err);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchData();
    // Refresh every 5 seconds
    const interval = setInterval(fetchData, 5000);
    return () => clearInterval(interval);
  }, [fetchData]);

  const handleRegenerateCode = async () => {
    try {
      const newCode = await regeneratePairingCode();
      setStatus(prev => prev ? { ...prev, pairing_code: newCode } : null);
    } catch (err) {
      console.error('Failed to regenerate code:', err);
    }
  };

  const handleCreateDb = async () => {
    if (!newDbName.trim()) return;
    try {
      await createDatabase(newDbName, newDbApp || 'unknown');
      setNewDbName('');
      setNewDbApp('');
      setShowAddDb(false);
      fetchData();
    } catch (err) {
      console.error('Failed to create database:', err);
    }
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  if (loading) {
    return (
      <div className="app loading">
        <div className="spinner"></div>
        <p>Initializing ADBA...</p>
      </div>
    );
  }

  return (
    <div className="app">
      {/* Header */}
      <header className="header">
        <div className="logo">
          <span className="logo-icon">🗄️</span>
          <h1>ADBA</h1>
        </div>
        <div className="status-badge" data-status={status?.running ? 'online' : 'offline'}>
          <span className="status-dot"></span>
          {status?.running ? 'Online' : 'Offline'}
        </div>
      </header>

      {/* Connection Panel */}
      <section className="panel connection-panel">
        <h2>📡 Connection Info</h2>
        <div className="connection-grid">
          <div className="connection-item">
            <label>Local IP</label>
            <div className="value-copy">
              <span>{status?.local_ip || 'N/A'}</span>
              <button onClick={() => copyToClipboard(status?.local_ip || '')}>📋</button>
            </div>
          </div>
          <div className="connection-item">
            <label>PostgreSQL Port</label>
            <div className="value-copy">
              <span>{status?.pg_port || 5433}</span>
              <button onClick={() => copyToClipboard(String(status?.pg_port || 5433))}>📋</button>
            </div>
          </div>
          <div className="connection-item pairing">
            <label>Pairing Code</label>
            <div className="pairing-code">
              <span className="code">{status?.pairing_code || '------'}</span>
              <button onClick={handleRegenerateCode} title="Regenerate">🔄</button>
              <button onClick={() => copyToClipboard(status?.pairing_code || '')} title="Copy">📋</button>
            </div>
          </div>
        </div>
        {connectionInfo && (
          <div className="connection-string">
            <label>Connection String</label>
            <div className="value-copy">
              <code>{connectionInfo.connection_string}</code>
              <button onClick={() => copyToClipboard(connectionInfo.connection_string)}>📋</button>
            </div>
          </div>
        )}
      </section>

      {/* Databases Panel */}
      <section className="panel databases-panel">
        <div className="panel-header">
          <h2>🗃️ Databases ({databases.length})</h2>
          <button className="add-btn" onClick={() => setShowAddDb(!showAddDb)}>
            {showAddDb ? '✕' : '+ Add'}
          </button>
        </div>

        {showAddDb && (
          <div className="add-db-form">
            <input
              type="text"
              placeholder="Database name"
              value={newDbName}
              onChange={(e) => setNewDbName(e.target.value)}
            />
            <input
              type="text"
              placeholder="Client app (optional)"
              value={newDbApp}
              onChange={(e) => setNewDbApp(e.target.value)}
            />
            <button onClick={handleCreateDb} disabled={!newDbName.trim()}>Create</button>
          </div>
        )}

        <div className="databases-list">
          {databases.length === 0 ? (
            <div className="empty-state">
              <span>📭</span>
              <p>No databases yet</p>
              <small>Create one or let a client app connect</small>
            </div>
          ) : (
            databases.map(db => (
              <div key={db.id} className="database-card">
                <div className="db-header">
                  <span className="db-name">{db.name}</span>
                  <span className={`db-status status-${db.status.toLowerCase()}`}>
                    {db.status}
                  </span>
                </div>
                <div className="db-meta">
                  <span>📱 {db.client_app}</span>
                  <span>📊 {db.tables_count} tables</span>
                  <span>💾 {formatBytes(db.size_bytes)}</span>
                </div>
              </div>
            ))
          )}
        </div>
      </section>

      {/* Stats Panel */}
      <section className="panel stats-panel">
        <h2>📈 Stats</h2>
        <div className="stats-grid">
          <div className="stat">
            <span className="stat-value">{status?.active_connections || 0}</span>
            <span className="stat-label">Active Connections</span>
          </div>
          <div className="stat">
            <span className="stat-value">{status?.databases_count || 0}</span>
            <span className="stat-label">Databases</span>
          </div>
        </div>
      </section>

      {/* Footer */}
      <footer className="footer">
        <p>ADBA v0.1.0 • PostgreSQL Wire Protocol • SurrealDB Engine</p>
      </footer>
    </div>
  );
}

function formatBytes(bytes: number): string {
  if (bytes === 0) return '0 B';
  const k = 1024;
  const sizes = ['B', 'KB', 'MB', 'GB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
}

export default App;
