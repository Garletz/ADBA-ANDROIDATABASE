/**
 * ADBA Frontend API
 * TypeScript bindings for Tauri Rust commands
 */

import { invoke } from '@tauri-apps/api/core';

// ============================================================================
// Types
// ============================================================================

export interface ServerStatus {
  running: boolean;
  pg_port: number;
  databases_count: number;
  active_connections: number;
  pairing_code: string;
  local_ip: string | null;
}

export interface DatabaseInfo {
  id: string;
  name: string;
  client_app: string;
  created_at: number;
  size_bytes: number;
  tables_count: number;
  status: 'Active' | 'Syncing' | 'Offline' | 'Error';
}

export interface ConnectionInfo {
  host: string;
  port: number;
  pairing_code: string;
  connection_string: string;
}

// ============================================================================
// API Functions
// ============================================================================

/**
 * Get current server status
 */
export async function getStatus(): Promise<ServerStatus> {
  return invoke('get_status');
}

/**
 * Get list of all databases
 */
export async function getDatabases(): Promise<DatabaseInfo[]> {
  return invoke('get_databases');
}

/**
 * Create a new database for a client app
 */
export async function createDatabase(name: string, clientApp: string): Promise<DatabaseInfo> {
  return invoke('create_database', { name, clientApp });
}

/**
 * Get pairing code for client connection
 */
export async function getPairingCode(): Promise<string> {
  return invoke('get_pairing_code');
}

/**
 * Regenerate pairing code
 */
export async function regeneratePairingCode(): Promise<string> {
  return invoke('regenerate_pairing_code');
}

/**
 * Get connection info for clients
 */
export async function getConnectionInfo(): Promise<ConnectionInfo> {
  return invoke('get_connection_info');
}
