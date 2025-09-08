export interface WebSocketMessage<T = any> {
  type: string;
  data: T;
  timestamp: string;
  user_id?: string;
}

export interface NoteUpdateMessage {
  note_id: string;
  title?: string;
  content?: string;
  version: number;
  user_id: string;
  operation: 'update' | 'create' | 'delete' | 'move';
}

export interface FolderUpdateMessage {
  folder_id: string;
  name?: string;
  parent_id?: string | null;
  user_id: string;
  operation: 'update' | 'create' | 'delete' | 'move';
}

export interface PresenceMessage {
  user_id: string;
  note_id?: string;
  folder_id?: string;
  status: 'online' | 'offline' | 'editing' | 'viewing';
}

export interface SyncMessage {
  entity_type: 'note' | 'folder';
  entity_id: string;
  operation: 'create' | 'update' | 'delete' | 'move';
  data: any;
  version?: number;
  conflict?: boolean;
}

export type WebSocketEventType =
  | 'note_update'
  | 'folder_update'
  | 'presence'
  | 'sync'
  | 'error'
  | 'connect'
  | 'disconnect';