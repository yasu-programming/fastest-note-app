import { useAuthStore } from '@/stores/authStore';
import { useContentStore } from '@/stores/contentStore';
import type {
  WebSocketMessage,
  NoteUpdateMessage,
  FolderUpdateMessage,
  PresenceMessage,
  SyncMessage,
  WebSocketEventType,
} from '@/types/websocket';

export type WebSocketEventHandler<T = any> = (data: T) => void;

export interface WebSocketOptions {
  url?: string;
  reconnectAttempts?: number;
  reconnectInterval?: number;
  heartbeatInterval?: number;
  connectionTimeout?: number;
}

export class WebSocketService {
  private ws: WebSocket | null = null;
  private url: string;
  private reconnectAttempts: number;
  private maxReconnectAttempts: number;
  private reconnectInterval: number;
  private heartbeatInterval: number;
  private connectionTimeout: number;
  private isConnected: boolean = false;
  private isConnecting: boolean = false;
  private shouldReconnect: boolean = true;
  private heartbeatTimer: NodeJS.Timeout | null = null;
  private connectionTimer: NodeJS.Timeout | null = null;
  private eventHandlers: Map<WebSocketEventType, Set<WebSocketEventHandler>> = new Map();

  constructor(options: WebSocketOptions = {}) {
    this.url = options.url || process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:3001/ws';
    this.maxReconnectAttempts = options.reconnectAttempts || 5;
    this.reconnectInterval = options.reconnectInterval || 3000;
    this.heartbeatInterval = options.heartbeatInterval || 30000;
    this.connectionTimeout = options.connectionTimeout || 10000;
    this.reconnectAttempts = 0;

    this.initializeEventHandlers();
  }

  private initializeEventHandlers(): void {
    this.on('note_update', this.handleNoteUpdate.bind(this));
    this.on('folder_update', this.handleFolderUpdate.bind(this));
    this.on('sync', this.handleSync.bind(this));
    this.on('presence', this.handlePresence.bind(this));
  }

  public connect(): Promise<void> {
    if (this.isConnected || this.isConnecting) {
      return Promise.resolve();
    }

    return new Promise((resolve, reject) => {
      try {
        this.isConnecting = true;
        const authState = useAuthStore.getState();
        
        if (!authState.accessToken) {
          reject(new Error('No access token available'));
          return;
        }

        const wsUrl = `${this.url}?token=${authState.accessToken}`;
        this.ws = new WebSocket(wsUrl);

        this.connectionTimer = setTimeout(() => {
          if (this.ws && this.ws.readyState === WebSocket.CONNECTING) {
            this.ws.close();
            reject(new Error('Connection timeout'));
          }
        }, this.connectionTimeout);

        this.ws.onopen = () => {
          console.log('WebSocket connected');
          this.isConnected = true;
          this.isConnecting = false;
          this.reconnectAttempts = 0;
          
          if (this.connectionTimer) {
            clearTimeout(this.connectionTimer);
            this.connectionTimer = null;
          }

          this.startHeartbeat();
          this.emit('connect', null);
          resolve();
        };

        this.ws.onclose = (event) => {
          console.log('WebSocket disconnected:', event.code, event.reason);
          this.isConnected = false;
          this.isConnecting = false;
          this.stopHeartbeat();
          
          if (this.connectionTimer) {
            clearTimeout(this.connectionTimer);
            this.connectionTimer = null;
          }

          this.emit('disconnect', { code: event.code, reason: event.reason });

          if (this.shouldReconnect && this.reconnectAttempts < this.maxReconnectAttempts) {
            setTimeout(() => {
              this.reconnectAttempts++;
              console.log(`Reconnecting... Attempt ${this.reconnectAttempts}`);
              this.connect().catch((error) => {
                console.error('Reconnection failed:', error);
              });
            }, this.reconnectInterval * this.reconnectAttempts);
          }
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          this.emit('error', { error });
          
          if (this.isConnecting) {
            reject(error);
          }
        };

        this.ws.onmessage = (event) => {
          try {
            const message: WebSocketMessage = JSON.parse(event.data);
            this.handleMessage(message);
          } catch (error) {
            console.error('Failed to parse WebSocket message:', error);
          }
        };
      } catch (error) {
        this.isConnecting = false;
        reject(error);
      }
    });
  }

  public disconnect(): void {
    this.shouldReconnect = false;
    
    if (this.ws) {
      this.ws.close();
    }
    
    this.stopHeartbeat();
    
    if (this.connectionTimer) {
      clearTimeout(this.connectionTimer);
      this.connectionTimer = null;
    }
  }

  public send<T>(type: WebSocketEventType, data: T): boolean {
    if (!this.isConnected || !this.ws) {
      console.warn('WebSocket not connected, cannot send message');
      return false;
    }

    try {
      const message: WebSocketMessage<T> = {
        type,
        data,
        timestamp: new Date().toISOString(),
      };

      this.ws.send(JSON.stringify(message));
      return true;
    } catch (error) {
      console.error('Failed to send WebSocket message:', error);
      return false;
    }
  }

  public on<T>(event: WebSocketEventType, handler: WebSocketEventHandler<T>): void {
    if (!this.eventHandlers.has(event)) {
      this.eventHandlers.set(event, new Set());
    }
    this.eventHandlers.get(event)!.add(handler);
  }

  public off<T>(event: WebSocketEventType, handler: WebSocketEventHandler<T>): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.delete(handler);
    }
  }

  private emit<T>(event: WebSocketEventType, data: T): void {
    const handlers = this.eventHandlers.get(event);
    if (handlers) {
      handlers.forEach((handler) => {
        try {
          handler(data);
        } catch (error) {
          console.error(`Error in WebSocket event handler for ${event}:`, error);
        }
      });
    }
  }

  private handleMessage(message: WebSocketMessage): void {
    switch (message.type) {
      case 'ping':
        this.send('pong', { timestamp: message.data.timestamp });
        break;
      case 'pong':
        break;
      default:
        this.emit(message.type as WebSocketEventType, message.data);
    }
  }

  private handleNoteUpdate(data: NoteUpdateMessage): void {
    const contentStore = useContentStore.getState();
    const authStore = useAuthStore.getState();

    if (data.user_id === authStore.user?.id) {
      return;
    }

    switch (data.operation) {
      case 'create':
        contentStore.syncNotes();
        break;
      case 'update':
        const currentNote = contentStore.notes[data.note_id];
        if (currentNote && currentNote.version < data.version) {
          contentStore.syncNotes();
        }
        break;
      case 'delete':
        contentStore.removeNote(data.note_id);
        break;
      case 'move':
        contentStore.syncNotes();
        break;
    }
  }

  private handleFolderUpdate(data: FolderUpdateMessage): void {
    const contentStore = useContentStore.getState();
    const authStore = useAuthStore.getState();

    if (data.user_id === authStore.user?.id) {
      return;
    }

    switch (data.operation) {
      case 'create':
      case 'update':
      case 'delete':
      case 'move':
        contentStore.syncFolders();
        break;
    }
  }

  private handleSync(data: SyncMessage): void {
    const contentStore = useContentStore.getState();

    if (data.conflict) {
      contentStore.addOptimisticOp({
        id: `conflict_${Date.now()}`,
        type: data.operation,
        entityType: data.entity_type,
        entityId: data.entity_id,
        status: 'error',
        error: 'Sync conflict detected',
        newData: data.data,
      });
    } else {
      if (data.entity_type === 'note') {
        contentStore.syncNotes();
      } else if (data.entity_type === 'folder') {
        contentStore.syncFolders();
      }
    }
  }

  private handlePresence(data: PresenceMessage): void {
    console.log('User presence update:', data);
  }

  private startHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
    }

    this.heartbeatTimer = setInterval(() => {
      if (this.isConnected) {
        this.send('ping', { timestamp: Date.now() });
      }
    }, this.heartbeatInterval);
  }

  private stopHeartbeat(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  public getConnectionState(): 'connecting' | 'connected' | 'disconnected' {
    if (this.isConnecting) return 'connecting';
    if (this.isConnected) return 'connected';
    return 'disconnected';
  }

  public isReady(): boolean {
    return this.isConnected && this.ws?.readyState === WebSocket.OPEN;
  }
}

export const websocketService = new WebSocketService();

export const useWebSocket = () => {
  const connect = () => websocketService.connect();
  const disconnect = () => websocketService.disconnect();
  const send = websocketService.send.bind(websocketService);
  const on = websocketService.on.bind(websocketService);
  const off = websocketService.off.bind(websocketService);
  const isReady = () => websocketService.isReady();
  const getConnectionState = () => websocketService.getConnectionState();

  return {
    connect,
    disconnect,
    send,
    on,
    off,
    isReady,
    getConnectionState,
  };
};

if (typeof window !== 'undefined') {
  const initializeWebSocket = () => {
    const authStore = useAuthStore.getState();
    
    if (authStore.isAuthenticated && authStore.accessToken) {
      websocketService.connect().catch((error) => {
        console.error('Failed to initialize WebSocket:', error);
      });
    }
  };

  useAuthStore.subscribe(
    (state) => state.isAuthenticated,
    (isAuthenticated) => {
      if (isAuthenticated) {
        setTimeout(initializeWebSocket, 100);
      } else {
        websocketService.disconnect();
      }
    }
  );

  window.addEventListener('beforeunload', () => {
    websocketService.disconnect();
  });

  window.addEventListener('online', () => {
    const authStore = useAuthStore.getState();
    if (authStore.isAuthenticated) {
      websocketService.connect().catch((error) => {
        console.error('Failed to reconnect WebSocket when online:', error);
      });
    }
  });

  window.addEventListener('offline', () => {
    websocketService.disconnect();
  });
}