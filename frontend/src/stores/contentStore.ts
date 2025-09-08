import { create } from 'zustand';
import { immer } from 'zustand/middleware/immer';
import { api } from '@/services/api';
import type { Note, CreateNoteRequest, UpdateNoteRequest } from '@/types/note';
import type { Folder, CreateFolderRequest, UpdateFolderRequest } from '@/types/folder';

export interface OptimisticOperation {
  id: string;
  type: 'create' | 'update' | 'delete' | 'move';
  entityType: 'note' | 'folder';
  entityId: string;
  timestamp: number;
  originalData?: any;
  newData?: any;
  status: 'pending' | 'success' | 'error';
  error?: string;
}

export interface ContentState {
  notes: Record<string, Note>;
  folders: Record<string, Folder>;
  selectedNoteId: string | null;
  selectedFolderId: string | null;
  searchQuery: string;
  isLoading: boolean;
  error: string | null;
  optimisticOps: OptimisticOperation[];
}

export interface ContentActions {
  // Notes
  setNotes: (notes: Note[]) => void;
  addNote: (note: Note) => void;
  updateNote: (id: string, updates: Partial<Note>) => void;
  removeNote: (id: string) => void;
  selectNote: (id: string | null) => void;
  
  // Folders
  setFolders: (folders: Folder[]) => void;
  addFolder: (folder: Folder) => void;
  updateFolder: (id: string, updates: Partial<Folder>) => void;
  removeFolder: (id: string) => void;
  selectFolder: (id: string | null) => void;
  
  // Search
  setSearchQuery: (query: string) => void;
  
  // Loading & errors
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  
  // Optimistic operations
  addOptimisticOp: (op: Omit<OptimisticOperation, 'timestamp'>) => void;
  updateOptimisticOp: (id: string, updates: Partial<OptimisticOperation>) => void;
  removeOptimisticOp: (id: string) => void;
  clearOptimisticOps: () => void;
  
  // API operations with optimistic updates
  createNoteOptimistic: (data: CreateNoteRequest) => Promise<Note>;
  updateNoteOptimistic: (id: string, data: UpdateNoteRequest) => Promise<Note>;
  deleteNoteOptimistic: (id: string) => Promise<void>;
  moveNoteOptimistic: (id: string, folderId: string | null) => Promise<Note>;
  
  createFolderOptimistic: (data: CreateFolderRequest) => Promise<Folder>;
  updateFolderOptimistic: (id: string, data: UpdateFolderRequest) => Promise<Folder>;
  deleteFolderOptimistic: (id: string) => Promise<void>;
  moveFolderOptimistic: (id: string, parentId: string | null) => Promise<Folder>;
  
  // Sync operations
  syncNotes: () => Promise<void>;
  syncFolders: () => Promise<void>;
  resolveConflict: (opId: string, resolution: 'accept' | 'reject') => void;
}

export type ContentStore = ContentState & ContentActions;

const generateId = () => `temp_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

const initialState: ContentState = {
  notes: {},
  folders: {},
  selectedNoteId: null,
  selectedFolderId: null,
  searchQuery: '',
  isLoading: false,
  error: null,
  optimisticOps: [],
};

export const useContentStore = create<ContentStore>()(
  immer((set, get) => ({
    ...initialState,

    // Notes
    setNotes: (notes: Note[]) => {
      set((state) => {
        state.notes = {};
        notes.forEach((note) => {
          state.notes[note.id] = note;
        });
      });
    },

    addNote: (note: Note) => {
      set((state) => {
        state.notes[note.id] = note;
      });
    },

    updateNote: (id: string, updates: Partial<Note>) => {
      set((state) => {
        if (state.notes[id]) {
          Object.assign(state.notes[id], updates);
        }
      });
    },

    removeNote: (id: string) => {
      set((state) => {
        delete state.notes[id];
        if (state.selectedNoteId === id) {
          state.selectedNoteId = null;
        }
      });
    },

    selectNote: (id: string | null) => {
      set((state) => {
        state.selectedNoteId = id;
      });
    },

    // Folders
    setFolders: (folders: Folder[]) => {
      set((state) => {
        state.folders = {};
        folders.forEach((folder) => {
          state.folders[folder.id] = folder;
        });
      });
    },

    addFolder: (folder: Folder) => {
      set((state) => {
        state.folders[folder.id] = folder;
      });
    },

    updateFolder: (id: string, updates: Partial<Folder>) => {
      set((state) => {
        if (state.folders[id]) {
          Object.assign(state.folders[id], updates);
        }
      });
    },

    removeFolder: (id: string) => {
      set((state) => {
        delete state.folders[id];
        if (state.selectedFolderId === id) {
          state.selectedFolderId = null;
        }
      });
    },

    selectFolder: (id: string | null) => {
      set((state) => {
        state.selectedFolderId = id;
      });
    },

    // Search
    setSearchQuery: (query: string) => {
      set((state) => {
        state.searchQuery = query;
      });
    },

    // Loading & errors
    setLoading: (loading: boolean) => {
      set((state) => {
        state.isLoading = loading;
      });
    },

    setError: (error: string | null) => {
      set((state) => {
        state.error = error;
      });
    },

    // Optimistic operations
    addOptimisticOp: (op: Omit<OptimisticOperation, 'timestamp'>) => {
      set((state) => {
        state.optimisticOps.push({
          ...op,
          timestamp: Date.now(),
        });
      });
    },

    updateOptimisticOp: (id: string, updates: Partial<OptimisticOperation>) => {
      set((state) => {
        const op = state.optimisticOps.find((o) => o.id === id);
        if (op) {
          Object.assign(op, updates);
        }
      });
    },

    removeOptimisticOp: (id: string) => {
      set((state) => {
        state.optimisticOps = state.optimisticOps.filter((op) => op.id !== id);
      });
    },

    clearOptimisticOps: () => {
      set((state) => {
        state.optimisticOps = [];
      });
    },

    // Optimistic API operations - Notes
    createNoteOptimistic: async (data: CreateNoteRequest): Promise<Note> => {
      const tempId = generateId();
      const opId = generateId();
      
      const optimisticNote: Note = {
        id: tempId,
        title: data.title,
        content: data.content,
        folder_id: data.folder_id || null,
        user_id: 'current_user',
        version: 1,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      // Add optimistic update
      get().addNote(optimisticNote);
      get().addOptimisticOp({
        id: opId,
        type: 'create',
        entityType: 'note',
        entityId: tempId,
        newData: optimisticNote,
        status: 'pending',
      });

      try {
        const createdNote = await api.notes.create(data);
        
        get().removeNote(tempId);
        get().addNote(createdNote);
        get().updateOptimisticOp(opId, { status: 'success' });
        
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
        
        return createdNote;
      } catch (error: any) {
        get().removeNote(tempId);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    updateNoteOptimistic: async (id: string, data: UpdateNoteRequest): Promise<Note> => {
      const opId = generateId();
      const currentNote = get().notes[id];
      
      if (!currentNote) {
        throw new Error('Note not found');
      }

      const optimisticNote: Note = {
        ...currentNote,
        ...data,
        updated_at: new Date().toISOString(),
        version: data.version,
      };

      get().updateNote(id, optimisticNote);
      get().addOptimisticOp({
        id: opId,
        type: 'update',
        entityType: 'note',
        entityId: id,
        originalData: currentNote,
        newData: optimisticNote,
        status: 'pending',
      });

      try {
        const updatedNote = await api.notes.update(id, data);
        
        get().updateNote(id, updatedNote);
        get().updateOptimisticOp(opId, { status: 'success' });
        
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
        
        return updatedNote;
      } catch (error: any) {
        get().updateNote(id, currentNote);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    deleteNoteOptimistic: async (id: string): Promise<void> => {
      const opId = generateId();
      const currentNote = get().notes[id];
      
      if (!currentNote) {
        throw new Error('Note not found');
      }

      get().removeNote(id);
      get().addOptimisticOp({
        id: opId,
        type: 'delete',
        entityType: 'note',
        entityId: id,
        originalData: currentNote,
        status: 'pending',
      });

      try {
        await api.notes.delete(id);
        
        get().updateOptimisticOp(opId, { status: 'success' });
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
      } catch (error: any) {
        get().addNote(currentNote);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    moveNoteOptimistic: async (id: string, folderId: string | null): Promise<Note> => {
      const opId = generateId();
      const currentNote = get().notes[id];
      
      if (!currentNote) {
        throw new Error('Note not found');
      }

      const optimisticNote: Note = {
        ...currentNote,
        folder_id: folderId,
        updated_at: new Date().toISOString(),
      };

      get().updateNote(id, optimisticNote);
      get().addOptimisticOp({
        id: opId,
        type: 'move',
        entityType: 'note',
        entityId: id,
        originalData: currentNote,
        newData: optimisticNote,
        status: 'pending',
      });

      try {
        const movedNote = await api.notes.move(id, { folder_id: folderId });
        
        get().updateNote(id, movedNote);
        get().updateOptimisticOp(opId, { status: 'success' });
        
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
        
        return movedNote;
      } catch (error: any) {
        get().updateNote(id, currentNote);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    // Optimistic API operations - Folders
    createFolderOptimistic: async (data: CreateFolderRequest): Promise<Folder> => {
      const tempId = generateId();
      const opId = generateId();
      
      const optimisticFolder: Folder = {
        id: tempId,
        name: data.name,
        parent_id: data.parent_id || null,
        path: data.parent_id ? `/${data.name}` : data.name,
        user_id: 'current_user',
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      get().addFolder(optimisticFolder);
      get().addOptimisticOp({
        id: opId,
        type: 'create',
        entityType: 'folder',
        entityId: tempId,
        newData: optimisticFolder,
        status: 'pending',
      });

      try {
        const createdFolder = await api.folders.create(data);
        
        get().removeFolder(tempId);
        get().addFolder(createdFolder);
        get().updateOptimisticOp(opId, { status: 'success' });
        
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
        
        return createdFolder;
      } catch (error: any) {
        get().removeFolder(tempId);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    updateFolderOptimistic: async (id: string, data: UpdateFolderRequest): Promise<Folder> => {
      const opId = generateId();
      const currentFolder = get().folders[id];
      
      if (!currentFolder) {
        throw new Error('Folder not found');
      }

      const optimisticFolder: Folder = {
        ...currentFolder,
        ...data,
        updated_at: new Date().toISOString(),
      };

      get().updateFolder(id, optimisticFolder);
      get().addOptimisticOp({
        id: opId,
        type: 'update',
        entityType: 'folder',
        entityId: id,
        originalData: currentFolder,
        newData: optimisticFolder,
        status: 'pending',
      });

      try {
        const updatedFolder = await api.folders.update(id, data);
        
        get().updateFolder(id, updatedFolder);
        get().updateOptimisticOp(opId, { status: 'success' });
        
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
        
        return updatedFolder;
      } catch (error: any) {
        get().updateFolder(id, currentFolder);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    deleteFolderOptimistic: async (id: string): Promise<void> => {
      const opId = generateId();
      const currentFolder = get().folders[id];
      
      if (!currentFolder) {
        throw new Error('Folder not found');
      }

      get().removeFolder(id);
      get().addOptimisticOp({
        id: opId,
        type: 'delete',
        entityType: 'folder',
        entityId: id,
        originalData: currentFolder,
        status: 'pending',
      });

      try {
        await api.folders.delete(id);
        
        get().updateOptimisticOp(opId, { status: 'success' });
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
      } catch (error: any) {
        get().addFolder(currentFolder);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    moveFolderOptimistic: async (id: string, parentId: string | null): Promise<Folder> => {
      const opId = generateId();
      const currentFolder = get().folders[id];
      
      if (!currentFolder) {
        throw new Error('Folder not found');
      }

      const optimisticFolder: Folder = {
        ...currentFolder,
        parent_id: parentId,
        updated_at: new Date().toISOString(),
      };

      get().updateFolder(id, optimisticFolder);
      get().addOptimisticOp({
        id: opId,
        type: 'move',
        entityType: 'folder',
        entityId: id,
        originalData: currentFolder,
        newData: optimisticFolder,
        status: 'pending',
      });

      try {
        const movedFolder = await api.folders.move(id, { parent_id: parentId });
        
        get().updateFolder(id, movedFolder);
        get().updateOptimisticOp(opId, { status: 'success' });
        
        setTimeout(() => get().removeOptimisticOp(opId), 1000);
        
        return movedFolder;
      } catch (error: any) {
        get().updateFolder(id, currentFolder);
        get().updateOptimisticOp(opId, { 
          status: 'error', 
          error: error.message 
        });
        
        setTimeout(() => get().removeOptimisticOp(opId), 5000);
        throw error;
      }
    },

    // Sync operations
    syncNotes: async (): Promise<void> => {
      try {
        get().setLoading(true);
        const notes = await api.notes.list();
        get().setNotes(notes);
        get().setError(null);
      } catch (error: any) {
        get().setError(error.message);
        throw error;
      } finally {
        get().setLoading(false);
      }
    },

    syncFolders: async (): Promise<void> => {
      try {
        get().setLoading(true);
        const folders = await api.folders.list();
        get().setFolders(folders);
        get().setError(null);
      } catch (error: any) {
        get().setError(error.message);
        throw error;
      } finally {
        get().setLoading(false);
      }
    },

    resolveConflict: (opId: string, resolution: 'accept' | 'reject') => {
      const op = get().optimisticOps.find((o) => o.id === opId);
      if (!op) return;

      if (resolution === 'accept') {
        get().updateOptimisticOp(opId, { status: 'success' });
      } else {
        if (op.originalData) {
          if (op.entityType === 'note') {
            get().updateNote(op.entityId, op.originalData);
          } else {
            get().updateFolder(op.entityId, op.originalData);
          }
        }
        get().updateOptimisticOp(opId, { status: 'error', error: 'Rejected by user' });
      }

      setTimeout(() => get().removeOptimisticOp(opId), 1000);
    },
  }))
);

export const useContent = () => {
  const {
    notes,
    folders,
    selectedNoteId,
    selectedFolderId,
    searchQuery,
    isLoading,
    error,
    optimisticOps,
    selectNote,
    selectFolder,
    setSearchQuery,
    createNoteOptimistic,
    updateNoteOptimistic,
    deleteNoteOptimistic,
    moveNoteOptimistic,
    createFolderOptimistic,
    updateFolderOptimistic,
    deleteFolderOptimistic,
    moveFolderOptimistic,
    syncNotes,
    syncFolders,
    resolveConflict,
  } = useContentStore();

  const notesArray = Object.values(notes);
  const foldersArray = Object.values(folders);
  const selectedNote = selectedNoteId ? notes[selectedNoteId] : null;
  const selectedFolder = selectedFolderId ? folders[selectedFolderId] : null;

  return {
    notes: notesArray,
    folders: foldersArray,
    selectedNote,
    selectedFolder,
    searchQuery,
    isLoading,
    error,
    optimisticOps,
    selectNote,
    selectFolder,
    setSearchQuery,
    createNote: createNoteOptimistic,
    updateNote: updateNoteOptimistic,
    deleteNote: deleteNoteOptimistic,
    moveNote: moveNoteOptimistic,
    createFolder: createFolderOptimistic,
    updateFolder: updateFolderOptimistic,
    deleteFolder: deleteFolderOptimistic,
    moveFolder: moveFolderOptimistic,
    syncNotes,
    syncFolders,
    resolveConflict,
  };
};