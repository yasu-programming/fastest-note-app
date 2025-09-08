import { api } from '@/services/api';
import { offlineStorage, type OfflineOperation } from '@/services/offline';
import { useContentStore } from '@/stores/contentStore';
import { useAuthStore } from '@/stores/authStore';
import type { Note, CreateNoteRequest, UpdateNoteRequest } from '@/types/note';
import type { Folder, CreateFolderRequest, UpdateFolderRequest } from '@/types/folder';

export interface SyncQueueOptions {
  maxRetries?: number;
  retryDelayMs?: number;
  maxConcurrentSync?: number;
  syncIntervalMs?: number;
}

export interface SyncResult {
  success: boolean;
  error?: string;
  conflicts?: Array<{
    operationId: string;
    localData: any;
    remoteData: any;
    field: string;
  }>;
}

export interface SyncStats {
  pendingOperations: number;
  failedOperations: number;
  lastSyncTime: number | null;
  isOnline: boolean;
  isSyncing: boolean;
}

class SyncQueueService {
  private maxRetries: number;
  private retryDelayMs: number;
  private maxConcurrentSync: number;
  private syncIntervalMs: number;
  private syncTimer: NodeJS.Timeout | null = null;
  private activeSyncCount = 0;
  private isOnline = typeof navigator !== 'undefined' ? navigator.onLine : true;
  private isSyncing = false;
  private eventListeners: Map<string, Set<(data: any) => void>> = new Map();

  constructor(options: SyncQueueOptions = {}) {
    this.maxRetries = options.maxRetries || 3;
    this.retryDelayMs = options.retryDelayMs || 5000;
    this.maxConcurrentSync = options.maxConcurrentSync || 3;
    this.syncIntervalMs = options.syncIntervalMs || 30000;

    if (typeof window !== 'undefined') {
      this.initializeEventListeners();
      this.startPeriodicSync();
    }
  }

  private initializeEventListeners(): void {
    window.addEventListener('online', () => {
      this.isOnline = true;
      this.emit('online', { isOnline: true });
      this.processPendingOperations();
    });

    window.addEventListener('offline', () => {
      this.isOnline = false;
      this.emit('offline', { isOnline: false });
    });

    window.addEventListener('beforeunload', () => {
      this.stopPeriodicSync();
    });

    document.addEventListener('visibilitychange', () => {
      if (document.visibilityState === 'visible' && this.isOnline) {
        this.processPendingOperations();
      }
    });
  }

  private startPeriodicSync(): void {
    if (this.syncTimer) {
      clearInterval(this.syncTimer);
    }

    this.syncTimer = setInterval(() => {
      if (this.isOnline && !this.isSyncing) {
        this.processPendingOperations();
      }
    }, this.syncIntervalMs);
  }

  private stopPeriodicSync(): void {
    if (this.syncTimer) {
      clearInterval(this.syncTimer);
      this.syncTimer = null;
    }
  }

  public async queueOperation(operation: Omit<OfflineOperation, 'id' | 'timestamp' | 'status' | 'retryCount'>): Promise<string> {
    const id = `op_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    const fullOperation: OfflineOperation = {
      ...operation,
      id,
      timestamp: Date.now(),
      status: 'pending',
      retryCount: 0,
    };

    await offlineStorage.saveOperation(fullOperation);
    this.emit('operationQueued', { operation: fullOperation });

    if (this.isOnline && this.activeSyncCount < this.maxConcurrentSync) {
      setImmediate(() => this.processPendingOperations());
    }

    return id;
  }

  public async processPendingOperations(): Promise<void> {
    if (!this.isOnline || this.isSyncing) {
      return;
    }

    this.isSyncing = true;
    this.emit('syncStarted', {});

    try {
      const pendingOps = await offlineStorage.getOperations('pending');
      const failedOps = await offlineStorage.getOperations('failed');
      const allOps = [...pendingOps, ...failedOps].sort((a, b) => a.timestamp - b.timestamp);

      if (allOps.length === 0) {
        this.emit('syncCompleted', { success: true, processed: 0 });
        return;
      }

      let processed = 0;
      let failed = 0;

      for (const operation of allOps) {
        if (this.activeSyncCount >= this.maxConcurrentSync) {
          break;
        }

        if (operation.retryCount >= this.maxRetries) {
          continue;
        }

        this.activeSyncCount++;
        this.processOperation(operation)
          .then(() => {
            processed++;
            this.emit('operationSynced', { operation });
          })
          .catch((error) => {
            failed++;
            this.emit('operationFailed', { operation, error });
          })
          .finally(() => {
            this.activeSyncCount--;
          });
      }

      this.emit('syncCompleted', { success: failed === 0, processed, failed });
    } catch (error) {
      console.error('Error processing pending operations:', error);
      this.emit('syncError', { error });
    } finally {
      this.isSyncing = false;
    }
  }

  private async processOperation(operation: OfflineOperation): Promise<void> {
    try {
      await offlineStorage.updateOperation(operation.id, { 
        status: 'syncing',
        lastError: undefined 
      });

      const result = await this.executeOperation(operation);

      if (result.success) {
        await offlineStorage.updateOperation(operation.id, { status: 'synced' });
        
        setTimeout(() => {
          offlineStorage.deleteOperation(operation.id).catch(console.error);
        }, 60000);
      } else {
        throw new Error(result.error || 'Operation failed');
      }
    } catch (error: any) {
      console.error(`Failed to sync operation ${operation.id}:`, error);

      const newRetryCount = operation.retryCount + 1;
      const status = newRetryCount >= this.maxRetries ? 'failed' : 'pending';

      await offlineStorage.updateOperation(operation.id, {
        status,
        retryCount: newRetryCount,
        lastError: error.message,
      });

      if (status === 'pending') {
        setTimeout(() => {
          this.processPendingOperations();
        }, this.retryDelayMs * newRetryCount);
      }

      throw error;
    }
  }

  private async executeOperation(operation: OfflineOperation): Promise<SyncResult> {
    const { type, entityType, entityId, data, userId } = operation;

    try {
      if (entityType === 'note') {
        return await this.executeNoteOperation(type, entityId, data, userId);
      } else if (entityType === 'folder') {
        return await this.executeFolderOperation(type, entityId, data, userId);
      } else {
        throw new Error(`Unknown entity type: ${entityType}`);
      }
    } catch (error: any) {
      if (error.status === 409) {
        return await this.handleConflict(operation, error.data);
      }
      throw error;
    }
  }

  private async executeNoteOperation(type: string, entityId: string, data: any, userId: string): Promise<SyncResult> {
    switch (type) {
      case 'create':
        const createData: CreateNoteRequest = {
          title: data.title,
          content: data.content,
          folder_id: data.folder_id,
        };
        const createdNote = await api.notes.create(createData);
        
        const contentStore = useContentStore.getState();
        contentStore.removeNote(entityId);
        contentStore.addNote(createdNote);
        await offlineStorage.deleteNote(entityId);
        await offlineStorage.saveNote(createdNote);
        
        return { success: true };

      case 'update':
        const updateData: UpdateNoteRequest = {
          title: data.title,
          content: data.content,
          folder_id: data.folder_id,
          version: data.version,
        };
        const updatedNote = await api.notes.update(entityId, updateData);
        
        useContentStore.getState().updateNote(entityId, updatedNote);
        await offlineStorage.saveNote(updatedNote);
        
        return { success: true };

      case 'delete':
        await api.notes.delete(entityId);
        
        useContentStore.getState().removeNote(entityId);
        await offlineStorage.deleteNote(entityId);
        
        return { success: true };

      case 'move':
        const moveData = { folder_id: data.folder_id };
        const movedNote = await api.notes.move(entityId, moveData);
        
        useContentStore.getState().updateNote(entityId, movedNote);
        await offlineStorage.saveNote(movedNote);
        
        return { success: true };

      default:
        throw new Error(`Unknown note operation: ${type}`);
    }
  }

  private async executeFolderOperation(type: string, entityId: string, data: any, userId: string): Promise<SyncResult> {
    switch (type) {
      case 'create':
        const createData: CreateFolderRequest = {
          name: data.name,
          parent_id: data.parent_id,
        };
        const createdFolder = await api.folders.create(createData);
        
        const contentStore = useContentStore.getState();
        contentStore.removeFolder(entityId);
        contentStore.addFolder(createdFolder);
        await offlineStorage.deleteFolder(entityId);
        await offlineStorage.saveFolder(createdFolder);
        
        return { success: true };

      case 'update':
        const updateData: UpdateFolderRequest = {
          name: data.name,
          parent_id: data.parent_id,
        };
        const updatedFolder = await api.folders.update(entityId, updateData);
        
        useContentStore.getState().updateFolder(entityId, updatedFolder);
        await offlineStorage.saveFolder(updatedFolder);
        
        return { success: true };

      case 'delete':
        await api.folders.delete(entityId);
        
        useContentStore.getState().removeFolder(entityId);
        await offlineStorage.deleteFolder(entityId);
        
        return { success: true };

      case 'move':
        const moveData = { parent_id: data.parent_id };
        const movedFolder = await api.folders.move(entityId, moveData);
        
        useContentStore.getState().updateFolder(entityId, movedFolder);
        await offlineStorage.saveFolder(movedFolder);
        
        return { success: true };

      default:
        throw new Error(`Unknown folder operation: ${type}`);
    }
  }

  private async handleConflict(operation: OfflineOperation, conflictData: any): Promise<SyncResult> {
    const conflicts = [{
      operationId: operation.id,
      localData: operation.data,
      remoteData: conflictData,
      field: 'content',
    }];

    this.emit('conflict', { operation, conflicts });

    return {
      success: false,
      error: 'Conflict detected',
      conflicts,
    };
  }

  public async resolvePendingConflicts(): Promise<void> {
    const failedOps = await offlineStorage.getOperations('failed');
    const conflictOps = failedOps.filter(op => op.lastError?.includes('conflict'));

    for (const operation of conflictOps) {
      this.emit('conflict', { 
        operation, 
        conflicts: [{
          operationId: operation.id,
          localData: operation.data,
          remoteData: null,
          field: 'content',
        }]
      });
    }
  }

  public async retryOperation(operationId: string): Promise<void> {
    const operation = (await offlineStorage.getOperations()).find(op => op.id === operationId);
    
    if (!operation) {
      throw new Error('Operation not found');
    }

    await offlineStorage.updateOperation(operationId, { 
      status: 'pending',
      retryCount: 0,
      lastError: undefined,
    });

    if (this.isOnline) {
      this.processPendingOperations();
    }
  }

  public async cancelOperation(operationId: string): Promise<void> {
    const operation = (await offlineStorage.getOperations()).find(op => op.id === operationId);
    
    if (operation) {
      if (operation.type === 'create' && operation.entityType === 'note') {
        useContentStore.getState().removeNote(operation.entityId);
        await offlineStorage.deleteNote(operation.entityId);
      } else if (operation.type === 'create' && operation.entityType === 'folder') {
        useContentStore.getState().removeFolder(operation.entityId);
        await offlineStorage.deleteFolder(operation.entityId);
      }
    }

    await offlineStorage.deleteOperation(operationId);
    this.emit('operationCancelled', { operationId });
  }

  public async getSyncStats(): Promise<SyncStats> {
    const [pendingOps, failedOps] = await Promise.all([
      offlineStorage.getOperations('pending'),
      offlineStorage.getOperations('failed'),
    ]);

    const lastSyncTime = await offlineStorage.getMetadata('lastSyncTime');

    return {
      pendingOperations: pendingOps.length,
      failedOperations: failedOps.length,
      lastSyncTime,
      isOnline: this.isOnline,
      isSyncing: this.isSyncing,
    };
  }

  public async fullSync(): Promise<void> {
    if (!this.isOnline) {
      throw new Error('Cannot sync while offline');
    }

    this.emit('fullSyncStarted', {});

    try {
      const contentStore = useContentStore.getState();
      
      const [remoteNotes, remoteFolders] = await Promise.all([
        api.notes.list(),
        api.folders.list(),
      ]);

      contentStore.setNotes(remoteNotes);
      contentStore.setFolders(remoteFolders);

      await Promise.all([
        offlineStorage.saveNotes(remoteNotes),
        offlineStorage.saveFolders(remoteFolders),
        offlineStorage.setMetadata('lastSyncTime', Date.now()),
      ]);

      await this.processPendingOperations();

      await offlineStorage.clearSyncedOperations();

      this.emit('fullSyncCompleted', { 
        notesCount: remoteNotes.length,
        foldersCount: remoteFolders.length 
      });
    } catch (error) {
      this.emit('fullSyncError', { error });
      throw error;
    }
  }

  public async clearFailedOperations(): Promise<void> {
    const failedOps = await offlineStorage.getOperations('failed');
    
    for (const operation of failedOps) {
      await offlineStorage.deleteOperation(operation.id);
    }

    this.emit('failedOperationsCleared', { count: failedOps.length });
  }

  public on(event: string, handler: (data: any) => void): void {
    if (!this.eventListeners.has(event)) {
      this.eventListeners.set(event, new Set());
    }
    this.eventListeners.get(event)!.add(handler);
  }

  public off(event: string, handler: (data: any) => void): void {
    const handlers = this.eventListeners.get(event);
    if (handlers) {
      handlers.delete(handler);
    }
  }

  private emit(event: string, data: any): void {
    const handlers = this.eventListeners.get(event);
    if (handlers) {
      handlers.forEach(handler => {
        try {
          handler(data);
        } catch (error) {
          console.error(`Error in sync queue event handler for ${event}:`, error);
        }
      });
    }
  }

  public destroy(): void {
    this.stopPeriodicSync();
    this.eventListeners.clear();
  }
}

export const syncQueue = new SyncQueueService();

export const useSyncQueue = () => {
  const queueOperation = (operation: Omit<OfflineOperation, 'id' | 'timestamp' | 'status' | 'retryCount'>) => 
    syncQueue.queueOperation(operation);
  
  const processPendingOperations = () => syncQueue.processPendingOperations();
  const retryOperation = (operationId: string) => syncQueue.retryOperation(operationId);
  const cancelOperation = (operationId: string) => syncQueue.cancelOperation(operationId);
  const getSyncStats = () => syncQueue.getSyncStats();
  const fullSync = () => syncQueue.fullSync();
  const clearFailedOperations = () => syncQueue.clearFailedOperations();
  const resolvePendingConflicts = () => syncQueue.resolvePendingConflicts();

  const on = (event: string, handler: (data: any) => void) => syncQueue.on(event, handler);
  const off = (event: string, handler: (data: any) => void) => syncQueue.off(event, handler);

  return {
    queueOperation,
    processPendingOperations,
    retryOperation,
    cancelOperation,
    getSyncStats,
    fullSync,
    clearFailedOperations,
    resolvePendingConflicts,
    on,
    off,
  };
};

if (typeof window !== 'undefined') {
  useAuthStore.subscribe(
    (state) => state.isAuthenticated,
    (isAuthenticated) => {
      if (isAuthenticated) {
        setTimeout(() => {
          syncQueue.processPendingOperations().catch(console.error);
        }, 1000);
      }
    }
  );
}