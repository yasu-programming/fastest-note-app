import type { Note } from '@/types/note';
import type { Folder } from '@/types/folder';
import type { User } from '@/types/auth';

export interface OfflineOperation {
  id: string;
  type: 'create' | 'update' | 'delete' | 'move';
  entityType: 'note' | 'folder';
  entityId: string;
  data: any;
  timestamp: number;
  userId: string;
  status: 'pending' | 'syncing' | 'synced' | 'failed';
  retryCount: number;
  lastError?: string;
}

export interface OfflineData {
  notes: Record<string, Note>;
  folders: Record<string, Folder>;
  operations: OfflineOperation[];
  lastSync: number;
  version: number;
}

class OfflineStorageService {
  private dbName = 'FastestNoteApp';
  private dbVersion = 1;
  private db: IDBDatabase | null = null;

  async initialize(): Promise<void> {
    if (typeof window === 'undefined' || !('indexedDB' in window)) {
      console.warn('IndexedDB not available');
      return;
    }

    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.dbName, this.dbVersion);

      request.onerror = () => {
        reject(new Error('Failed to open IndexedDB'));
      };

      request.onsuccess = () => {
        this.db = request.result;
        
        this.db.onversionchange = () => {
          this.db?.close();
          console.log('Database version changed. Please reload the page.');
        };
        
        resolve();
      };

      request.onupgradeneeded = (event) => {
        const db = (event.target as IDBOpenDBRequest).result;

        if (!db.objectStoreNames.contains('notes')) {
          const notesStore = db.createObjectStore('notes', { keyPath: 'id' });
          notesStore.createIndex('folder_id', 'folder_id', { unique: false });
          notesStore.createIndex('updated_at', 'updated_at', { unique: false });
          notesStore.createIndex('created_at', 'created_at', { unique: false });
        }

        if (!db.objectStoreNames.contains('folders')) {
          const foldersStore = db.createObjectStore('folders', { keyPath: 'id' });
          foldersStore.createIndex('parent_id', 'parent_id', { unique: false });
          foldersStore.createIndex('path', 'path', { unique: false });
        }

        if (!db.objectStoreNames.contains('operations')) {
          const operationsStore = db.createObjectStore('operations', { keyPath: 'id' });
          operationsStore.createIndex('timestamp', 'timestamp', { unique: false });
          operationsStore.createIndex('status', 'status', { unique: false });
          operationsStore.createIndex('entityType', 'entityType', { unique: false });
        }

        if (!db.objectStoreNames.contains('metadata')) {
          db.createObjectStore('metadata', { keyPath: 'key' });
        }

        if (!db.objectStoreNames.contains('attachments')) {
          const attachmentsStore = db.createObjectStore('attachments', { keyPath: 'id' });
          attachmentsStore.createIndex('note_id', 'note_id', { unique: false });
        }
      };
    });
  }

  private async getObjectStore(storeName: string, mode: IDBTransactionMode = 'readonly'): Promise<IDBObjectStore> {
    if (!this.db) {
      await this.initialize();
    }

    if (!this.db) {
      throw new Error('Database not initialized');
    }

    const transaction = this.db.transaction([storeName], mode);
    return transaction.objectStore(storeName);
  }

  // Notes operations
  async saveNote(note: Note): Promise<void> {
    const store = await this.getObjectStore('notes', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.put({
        ...note,
        _offlineUpdated: Date.now(),
      });
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async saveNotes(notes: Note[]): Promise<void> {
    const store = await this.getObjectStore('notes', 'readwrite');
    const transaction = store.transaction;

    return new Promise((resolve, reject) => {
      let completed = 0;
      let hasError = false;

      const handleResult = () => {
        completed++;
        if (completed === notes.length) {
          if (hasError) {
            reject(new Error('Some notes failed to save'));
          } else {
            resolve();
          }
        }
      };

      notes.forEach(note => {
        const request = store.put({
          ...note,
          _offlineUpdated: Date.now(),
        });
        
        request.onsuccess = handleResult;
        request.onerror = () => {
          hasError = true;
          handleResult();
        };
      });

      transaction.onerror = () => reject(transaction.error);
    });
  }

  async getNote(id: string): Promise<Note | null> {
    const store = await this.getObjectStore('notes');
    return new Promise((resolve, reject) => {
      const request = store.get(id);
      request.onsuccess = () => {
        const result = request.result;
        if (result) {
          const { _offlineUpdated, ...note } = result;
          resolve(note as Note);
        } else {
          resolve(null);
        }
      };
      request.onerror = () => reject(request.error);
    });
  }

  async getNotes(folderId?: string): Promise<Note[]> {
    const store = await this.getObjectStore('notes');
    return new Promise((resolve, reject) => {
      let request: IDBRequest;

      if (folderId) {
        const index = store.index('folder_id');
        request = index.getAll(folderId);
      } else {
        request = store.getAll();
      }

      request.onsuccess = () => {
        const results = request.result.map((item: any) => {
          const { _offlineUpdated, ...note } = item;
          return note as Note;
        });
        resolve(results);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async deleteNote(id: string): Promise<void> {
    const store = await this.getObjectStore('notes', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.delete(id);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async searchNotes(query: string): Promise<Note[]> {
    const notes = await this.getNotes();
    const lowercaseQuery = query.toLowerCase();
    
    return notes.filter(note => 
      note.title.toLowerCase().includes(lowercaseQuery) ||
      note.content.toLowerCase().includes(lowercaseQuery)
    );
  }

  // Folders operations
  async saveFolder(folder: Folder): Promise<void> {
    const store = await this.getObjectStore('folders', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.put({
        ...folder,
        _offlineUpdated: Date.now(),
      });
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async saveFolders(folders: Folder[]): Promise<void> {
    const store = await this.getObjectStore('folders', 'readwrite');
    const transaction = store.transaction;

    return new Promise((resolve, reject) => {
      let completed = 0;
      let hasError = false;

      const handleResult = () => {
        completed++;
        if (completed === folders.length) {
          if (hasError) {
            reject(new Error('Some folders failed to save'));
          } else {
            resolve();
          }
        }
      };

      folders.forEach(folder => {
        const request = store.put({
          ...folder,
          _offlineUpdated: Date.now(),
        });
        
        request.onsuccess = handleResult;
        request.onerror = () => {
          hasError = true;
          handleResult();
        };
      });

      transaction.onerror = () => reject(transaction.error);
    });
  }

  async getFolder(id: string): Promise<Folder | null> {
    const store = await this.getObjectStore('folders');
    return new Promise((resolve, reject) => {
      const request = store.get(id);
      request.onsuccess = () => {
        const result = request.result;
        if (result) {
          const { _offlineUpdated, ...folder } = result;
          resolve(folder as Folder);
        } else {
          resolve(null);
        }
      };
      request.onerror = () => reject(request.error);
    });
  }

  async getFolders(): Promise<Folder[]> {
    const store = await this.getObjectStore('folders');
    return new Promise((resolve, reject) => {
      const request = store.getAll();
      request.onsuccess = () => {
        const results = request.result.map((item: any) => {
          const { _offlineUpdated, ...folder } = item;
          return folder as Folder;
        });
        resolve(results);
      };
      request.onerror = () => reject(request.error);
    });
  }

  async deleteFolder(id: string): Promise<void> {
    const store = await this.getObjectStore('folders', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.delete(id);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  // Operations queue
  async saveOperation(operation: OfflineOperation): Promise<void> {
    const store = await this.getObjectStore('operations', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.put(operation);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async getOperations(status?: OfflineOperation['status']): Promise<OfflineOperation[]> {
    const store = await this.getObjectStore('operations');
    return new Promise((resolve, reject) => {
      let request: IDBRequest;

      if (status) {
        const index = store.index('status');
        request = index.getAll(status);
      } else {
        request = store.getAll();
      }

      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }

  async updateOperation(id: string, updates: Partial<OfflineOperation>): Promise<void> {
    const store = await this.getObjectStore('operations', 'readwrite');
    return new Promise((resolve, reject) => {
      const getRequest = store.get(id);
      getRequest.onsuccess = () => {
        const operation = getRequest.result;
        if (operation) {
          const updatedOperation = { ...operation, ...updates };
          const putRequest = store.put(updatedOperation);
          putRequest.onsuccess = () => resolve();
          putRequest.onerror = () => reject(putRequest.error);
        } else {
          reject(new Error('Operation not found'));
        }
      };
      getRequest.onerror = () => reject(getRequest.error);
    });
  }

  async deleteOperation(id: string): Promise<void> {
    const store = await this.getObjectStore('operations', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.delete(id);
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async clearSyncedOperations(): Promise<void> {
    const operations = await this.getOperations('synced');
    const store = await this.getObjectStore('operations', 'readwrite');
    
    return new Promise((resolve, reject) => {
      let completed = 0;
      let hasError = false;

      const handleResult = () => {
        completed++;
        if (completed === operations.length) {
          if (hasError) {
            reject(new Error('Some operations failed to delete'));
          } else {
            resolve();
          }
        }
      };

      operations.forEach(op => {
        const request = store.delete(op.id);
        request.onsuccess = handleResult;
        request.onerror = () => {
          hasError = true;
          handleResult();
        };
      });

      if (operations.length === 0) {
        resolve();
      }
    });
  }

  // Metadata operations
  async setMetadata(key: string, value: any): Promise<void> {
    const store = await this.getObjectStore('metadata', 'readwrite');
    return new Promise((resolve, reject) => {
      const request = store.put({ key, value, timestamp: Date.now() });
      request.onsuccess = () => resolve();
      request.onerror = () => reject(request.error);
    });
  }

  async getMetadata(key: string): Promise<any> {
    const store = await this.getObjectStore('metadata');
    return new Promise((resolve, reject) => {
      const request = store.get(key);
      request.onsuccess = () => {
        const result = request.result;
        resolve(result ? result.value : null);
      };
      request.onerror = () => reject(request.error);
    });
  }

  // Utility methods
  async getStorageInfo(): Promise<{
    notesCount: number;
    foldersCount: number;
    operationsCount: number;
    estimatedSize: number;
  }> {
    const [notesCount, foldersCount, operationsCount] = await Promise.all([
      this.getCount('notes'),
      this.getCount('folders'),
      this.getCount('operations'),
    ]);

    const estimatedSize = await this.getEstimatedSize();

    return {
      notesCount,
      foldersCount,
      operationsCount,
      estimatedSize,
    };
  }

  private async getCount(storeName: string): Promise<number> {
    const store = await this.getObjectStore(storeName);
    return new Promise((resolve, reject) => {
      const request = store.count();
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
  }

  private async getEstimatedSize(): Promise<number> {
    if ('storage' in navigator && 'estimate' in navigator.storage) {
      try {
        const estimate = await navigator.storage.estimate();
        return estimate.usage || 0;
      } catch (error) {
        console.warn('Could not estimate storage size:', error);
      }
    }
    return 0;
  }

  async clearAll(): Promise<void> {
    if (!this.db) return;

    const storeNames = ['notes', 'folders', 'operations', 'metadata', 'attachments'];
    const transaction = this.db.transaction(storeNames, 'readwrite');

    return new Promise((resolve, reject) => {
      let completed = 0;
      let hasError = false;

      const handleResult = () => {
        completed++;
        if (completed === storeNames.length) {
          if (hasError) {
            reject(new Error('Failed to clear all stores'));
          } else {
            resolve();
          }
        }
      };

      storeNames.forEach(storeName => {
        const request = transaction.objectStore(storeName).clear();
        request.onsuccess = handleResult;
        request.onerror = () => {
          hasError = true;
          handleResult();
        };
      });

      transaction.onerror = () => reject(transaction.error);
    });
  }

  async close(): Promise<void> {
    if (this.db) {
      this.db.close();
      this.db = null;
    }
  }

  // Export/Import for debugging
  async exportData(): Promise<OfflineData> {
    const [notes, folders, operations] = await Promise.all([
      this.getNotes(),
      this.getFolders(),
      this.getOperations(),
    ]);

    const notesMap = notes.reduce((acc, note) => {
      acc[note.id] = note;
      return acc;
    }, {} as Record<string, Note>);

    const foldersMap = folders.reduce((acc, folder) => {
      acc[folder.id] = folder;
      return acc;
    }, {} as Record<string, Folder>);

    const lastSync = await this.getMetadata('lastSync') || 0;

    return {
      notes: notesMap,
      folders: foldersMap,
      operations,
      lastSync,
      version: this.dbVersion,
    };
  }

  async importData(data: OfflineData): Promise<void> {
    await this.clearAll();
    
    const notes = Object.values(data.notes);
    const folders = Object.values(data.folders);

    await Promise.all([
      this.saveNotes(notes),
      this.saveFolders(folders),
      ...data.operations.map(op => this.saveOperation(op)),
      this.setMetadata('lastSync', data.lastSync),
    ]);
  }
}

export const offlineStorage = new OfflineStorageService();

export const useOfflineStorage = () => {
  const saveNote = (note: Note) => offlineStorage.saveNote(note);
  const saveNotes = (notes: Note[]) => offlineStorage.saveNotes(notes);
  const getNote = (id: string) => offlineStorage.getNote(id);
  const getNotes = (folderId?: string) => offlineStorage.getNotes(folderId);
  const deleteNote = (id: string) => offlineStorage.deleteNote(id);
  const searchNotes = (query: string) => offlineStorage.searchNotes(query);

  const saveFolder = (folder: Folder) => offlineStorage.saveFolder(folder);
  const saveFolders = (folders: Folder[]) => offlineStorage.saveFolders(folders);
  const getFolder = (id: string) => offlineStorage.getFolder(id);
  const getFolders = () => offlineStorage.getFolders();
  const deleteFolder = (id: string) => offlineStorage.deleteFolder(id);

  const saveOperation = (operation: OfflineOperation) => offlineStorage.saveOperation(operation);
  const getOperations = (status?: OfflineOperation['status']) => offlineStorage.getOperations(status);
  const updateOperation = (id: string, updates: Partial<OfflineOperation>) => offlineStorage.updateOperation(id, updates);
  const deleteOperation = (id: string) => offlineStorage.deleteOperation(id);

  const getStorageInfo = () => offlineStorage.getStorageInfo();
  const clearAll = () => offlineStorage.clearAll();
  const exportData = () => offlineStorage.exportData();
  const importData = (data: OfflineData) => offlineStorage.importData(data);

  return {
    saveNote,
    saveNotes,
    getNote,
    getNotes,
    deleteNote,
    searchNotes,
    saveFolder,
    saveFolders,
    getFolder,
    getFolders,
    deleteFolder,
    saveOperation,
    getOperations,
    updateOperation,
    deleteOperation,
    getStorageInfo,
    clearAll,
    exportData,
    importData,
  };
};

if (typeof window !== 'undefined') {
  offlineStorage.initialize().catch((error) => {
    console.error('Failed to initialize offline storage:', error);
  });
}