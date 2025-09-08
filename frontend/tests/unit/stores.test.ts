import { act } from '@testing-library/react';
import { useAuthStore } from '@/stores/authStore';
import { useContentStore } from '@/stores/contentStore';
import { api } from '@/services/api';
import type { User, AuthResponse } from '@/types/auth';
import type { Note } from '@/types/note';
import type { Folder } from '@/types/folder';

// Mock the API
jest.mock('@/services/api', () => ({
  api: {
    auth: {
      login: jest.fn(),
      register: jest.fn(),
      me: jest.fn(),
      refresh: jest.fn(),
    },
    notes: {
      create: jest.fn(),
      update: jest.fn(),
      delete: jest.fn(),
      move: jest.fn(),
      list: jest.fn(),
    },
    folders: {
      create: jest.fn(),
      update: jest.fn(),
      delete: jest.fn(),
      move: jest.fn(),
      list: jest.fn(),
    },
    setAuthToken: jest.fn(),
    clearAuth: jest.fn(),
  },
}));

const mockApi = api as jest.Mocked<typeof api>;

// Mock localStorage
const localStorageMock = {
  getItem: jest.fn(),
  setItem: jest.fn(),
  removeItem: jest.fn(),
  clear: jest.fn(),
};
Object.defineProperty(window, 'localStorage', { value: localStorageMock });

// Mock data
const mockUser: User = {
  id: 'user-1',
  email: 'test@example.com',
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

const mockAuthResponse: AuthResponse = {
  user: mockUser,
  access_token: 'mock-access-token',
  refresh_token: 'mock-refresh-token',
  expires_in: 3600,
};

const mockNote: Note = {
  id: 'note-1',
  title: 'Test Note',
  content: 'Test content',
  folder_id: null,
  user_id: 'user-1',
  version: 1,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

const mockFolder: Folder = {
  id: 'folder-1',
  name: 'Test Folder',
  parent_id: null,
  path: 'Test Folder',
  user_id: 'user-1',
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

describe('AuthStore', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    localStorageMock.getItem.mockReturnValue(null);
    useAuthStore.getState().logout();
  });

  it('initializes with default state', () => {
    const state = useAuthStore.getState();
    
    expect(state.user).toBeNull();
    expect(state.isAuthenticated).toBe(false);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('handles successful login', async () => {
    mockApi.auth.login.mockResolvedValue(mockAuthResponse);

    const store = useAuthStore.getState();
    
    await act(async () => {
      await store.login({ email: 'test@example.com', password: 'password' });
    });

    const newState = useAuthStore.getState();
    
    expect(newState.user).toEqual(mockUser);
    expect(newState.isAuthenticated).toBe(true);
    expect(newState.isLoading).toBe(false);
    expect(newState.error).toBeNull();
    expect(mockApi.setAuthToken).toHaveBeenCalledWith(
      mockAuthResponse.access_token,
      mockAuthResponse.refresh_token
    );
  });

  it('handles login failure', async () => {
    const error = new Error('Invalid credentials');
    mockApi.auth.login.mockRejectedValue(error);

    const store = useAuthStore.getState();
    
    await act(async () => {
      try {
        await store.login({ email: 'test@example.com', password: 'wrong' });
      } catch (e) {
        // Expected to throw
      }
    });

    const newState = useAuthStore.getState();
    
    expect(newState.user).toBeNull();
    expect(newState.isAuthenticated).toBe(false);
    expect(newState.isLoading).toBe(false);
    expect(newState.error).toBe('Invalid credentials');
  });

  it('handles successful registration', async () => {
    mockApi.auth.register.mockResolvedValue(mockAuthResponse);

    const store = useAuthStore.getState();
    
    await act(async () => {
      await store.register({ email: 'test@example.com', password: 'password' });
    });

    const newState = useAuthStore.getState();
    
    expect(newState.user).toEqual(mockUser);
    expect(newState.isAuthenticated).toBe(true);
    expect(newState.isLoading).toBe(false);
    expect(newState.error).toBeNull();
  });

  it('handles logout', () => {
    // Set up authenticated state
    const store = useAuthStore.getState();
    useAuthStore.setState({
      user: mockUser,
      isAuthenticated: true,
      accessToken: 'token',
      refreshToken: 'refresh',
    });

    act(() => {
      store.logout();
    });

    const newState = useAuthStore.getState();
    
    expect(newState.user).toBeNull();
    expect(newState.isAuthenticated).toBe(false);
    expect(newState.accessToken).toBeNull();
    expect(newState.refreshToken).toBeNull();
    expect(mockApi.clearAuth).toHaveBeenCalled();
  });

  it('handles token refresh', async () => {
    const refreshResponse = {
      access_token: 'new-access-token',
      refresh_token: 'new-refresh-token',
      expires_in: 3600,
    };
    
    mockApi.auth.refresh.mockResolvedValue(refreshResponse);
    
    // Set up state with refresh token
    useAuthStore.setState({
      refreshToken: 'old-refresh-token',
    });

    const store = useAuthStore.getState();
    
    await act(async () => {
      await store.refreshAuth();
    });

    const newState = useAuthStore.getState();
    
    expect(newState.accessToken).toBe('new-access-token');
    expect(newState.refreshToken).toBe('new-refresh-token');
    expect(mockApi.setAuthToken).toHaveBeenCalledWith(
      'new-access-token',
      'new-refresh-token'
    );
  });

  it('handles auth initialization', async () => {
    localStorageMock.getItem
      .mockReturnValueOnce('stored-access-token')
      .mockReturnValueOnce('stored-refresh-token');
    
    mockApi.auth.me.mockResolvedValue(mockUser);

    const store = useAuthStore.getState();
    
    await act(async () => {
      await store.initializeAuth();
    });

    const newState = useAuthStore.getState();
    
    expect(newState.user).toEqual(mockUser);
    expect(newState.isAuthenticated).toBe(true);
    expect(mockApi.setAuthToken).toHaveBeenCalledWith(
      'stored-access-token',
      'stored-refresh-token'
    );
  });

  it('clears error state', () => {
    useAuthStore.setState({ error: 'Some error' });

    const store = useAuthStore.getState();
    
    act(() => {
      store.clearError();
    });

    const newState = useAuthStore.getState();
    expect(newState.error).toBeNull();
  });

  it('sets loading state', () => {
    const store = useAuthStore.getState();
    
    act(() => {
      store.setLoading(true);
    });

    expect(useAuthStore.getState().isLoading).toBe(true);

    act(() => {
      store.setLoading(false);
    });

    expect(useAuthStore.getState().isLoading).toBe(false);
  });
});

describe('ContentStore', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    // Reset store to initial state
    useContentStore.getState().setNotes([]);
    useContentStore.getState().setFolders([]);
    useContentStore.getState().clearOptimisticOps();
  });

  it('initializes with default state', () => {
    const state = useContentStore.getState();
    
    expect(state.notes).toEqual({});
    expect(state.folders).toEqual({});
    expect(state.selectedNoteId).toBeNull();
    expect(state.selectedFolderId).toBeNull();
    expect(state.searchQuery).toBe('');
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
    expect(state.optimisticOps).toEqual([]);
  });

  it('manages notes state', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setNotes([mockNote]);
    });

    expect(useContentStore.getState().notes).toEqual({
      [mockNote.id]: mockNote,
    });

    act(() => {
      store.addNote({ ...mockNote, id: 'note-2', title: 'New Note' });
    });

    const state = useContentStore.getState();
    expect(Object.keys(state.notes)).toHaveLength(2);
    expect(state.notes['note-2'].title).toBe('New Note');
  });

  it('updates notes', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setNotes([mockNote]);
      store.updateNote(mockNote.id, { title: 'Updated Title' });
    });

    const state = useContentStore.getState();
    expect(state.notes[mockNote.id].title).toBe('Updated Title');
  });

  it('removes notes', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setNotes([mockNote]);
      store.removeNote(mockNote.id);
    });

    const state = useContentStore.getState();
    expect(state.notes[mockNote.id]).toBeUndefined();
  });

  it('selects notes', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.selectNote(mockNote.id);
    });

    expect(useContentStore.getState().selectedNoteId).toBe(mockNote.id);

    act(() => {
      store.selectNote(null);
    });

    expect(useContentStore.getState().selectedNoteId).toBeNull();
  });

  it('manages folders state', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setFolders([mockFolder]);
    });

    expect(useContentStore.getState().folders).toEqual({
      [mockFolder.id]: mockFolder,
    });
  });

  it('handles search query', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setSearchQuery('test query');
    });

    expect(useContentStore.getState().searchQuery).toBe('test query');
  });

  it('manages loading state', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setLoading(true);
    });

    expect(useContentStore.getState().isLoading).toBe(true);
  });

  it('manages error state', () => {
    const store = useContentStore.getState();
    
    act(() => {
      store.setError('Test error');
    });

    expect(useContentStore.getState().error).toBe('Test error');

    act(() => {
      store.setError(null);
    });

    expect(useContentStore.getState().error).toBeNull();
  });

  it('creates note optimistically', async () => {
    mockApi.notes.create.mockResolvedValue(mockNote);

    const store = useContentStore.getState();
    
    await act(async () => {
      const result = await store.createNoteOptimistic({
        title: 'New Note',
        content: 'New content',
      });
      expect(result).toEqual(mockNote);
    });

    // Should have called API
    expect(mockApi.notes.create).toHaveBeenCalledWith({
      title: 'New Note',
      content: 'New content',
    });

    // Should have added note to state
    const state = useContentStore.getState();
    expect(state.notes[mockNote.id]).toEqual(mockNote);
  });

  it('handles optimistic create failure', async () => {
    const error = new Error('Create failed');
    mockApi.notes.create.mockRejectedValue(error);

    const store = useContentStore.getState();
    
    await act(async () => {
      try {
        await store.createNoteOptimistic({
          title: 'Failed Note',
          content: 'Failed content',
        });
      } catch (e) {
        expect(e).toBe(error);
      }
    });

    // Optimistic operation should be marked as error
    const state = useContentStore.getState();
    const errorOp = state.optimisticOps.find(op => op.status === 'error');
    expect(errorOp).toBeDefined();
  });

  it('updates note optimistically', async () => {
    const updatedNote = { ...mockNote, title: 'Updated Title', version: 2 };
    mockApi.notes.update.mockResolvedValue(updatedNote);

    const store = useContentStore.getState();
    
    // Add initial note
    act(() => {
      store.setNotes([mockNote]);
    });

    await act(async () => {
      const result = await store.updateNoteOptimistic(mockNote.id, {
        title: 'Updated Title',
        version: 2,
      });
      expect(result).toEqual(updatedNote);
    });

    expect(mockApi.notes.update).toHaveBeenCalledWith(mockNote.id, {
      title: 'Updated Title',
      version: 2,
    });

    const state = useContentStore.getState();
    expect(state.notes[mockNote.id]).toEqual(updatedNote);
  });

  it('deletes note optimistically', async () => {
    mockApi.notes.delete.mockResolvedValue(undefined);

    const store = useContentStore.getState();
    
    // Add initial note
    act(() => {
      store.setNotes([mockNote]);
    });

    await act(async () => {
      await store.deleteNoteOptimistic(mockNote.id);
    });

    expect(mockApi.notes.delete).toHaveBeenCalledWith(mockNote.id);

    const state = useContentStore.getState();
    expect(state.notes[mockNote.id]).toBeUndefined();
  });

  it('moves note optimistically', async () => {
    const movedNote = { ...mockNote, folder_id: 'folder-1' };
    mockApi.notes.move.mockResolvedValue(movedNote);

    const store = useContentStore.getState();
    
    act(() => {
      store.setNotes([mockNote]);
    });

    await act(async () => {
      const result = await store.moveNoteOptimistic(mockNote.id, 'folder-1');
      expect(result).toEqual(movedNote);
    });

    expect(mockApi.notes.move).toHaveBeenCalledWith(mockNote.id, {
      folder_id: 'folder-1',
    });
  });

  it('syncs notes', async () => {
    const notes = [mockNote, { ...mockNote, id: 'note-2' }];
    mockApi.notes.list.mockResolvedValue(notes);

    const store = useContentStore.getState();
    
    await act(async () => {
      await store.syncNotes();
    });

    expect(mockApi.notes.list).toHaveBeenCalled();

    const state = useContentStore.getState();
    expect(Object.keys(state.notes)).toHaveLength(2);
    expect(state.isLoading).toBe(false);
    expect(state.error).toBeNull();
  });

  it('syncs folders', async () => {
    const folders = [mockFolder, { ...mockFolder, id: 'folder-2' }];
    mockApi.folders.list.mockResolvedValue(folders);

    const store = useContentStore.getState();
    
    await act(async () => {
      await store.syncFolders();
    });

    expect(mockApi.folders.list).toHaveBeenCalled();

    const state = useContentStore.getState();
    expect(Object.keys(state.folders)).toHaveLength(2);
  });

  it('manages optimistic operations', () => {
    const store = useContentStore.getState();
    const mockOp = {
      id: 'op-1',
      type: 'create' as const,
      entityType: 'note' as const,
      entityId: 'note-1',
      newData: mockNote,
      status: 'pending' as const,
    };

    act(() => {
      store.addOptimisticOp(mockOp);
    });

    let state = useContentStore.getState();
    expect(state.optimisticOps).toHaveLength(1);
    expect(state.optimisticOps[0].id).toBe('op-1');

    act(() => {
      store.updateOptimisticOp('op-1', { status: 'success' });
    });

    state = useContentStore.getState();
    expect(state.optimisticOps[0].status).toBe('success');

    act(() => {
      store.removeOptimisticOp('op-1');
    });

    state = useContentStore.getState();
    expect(state.optimisticOps).toHaveLength(0);
  });

  it('resolves conflicts', () => {
    const store = useContentStore.getState();
    const conflictOp = {
      id: 'conflict-op',
      type: 'update' as const,
      entityType: 'note' as const,
      entityId: mockNote.id,
      originalData: mockNote,
      newData: { ...mockNote, title: 'Conflicted Title' },
      status: 'error' as const,
    };

    act(() => {
      store.setNotes([mockNote]);
      store.addOptimisticOp(conflictOp);
    });

    // Accept resolution
    act(() => {
      store.resolveConflict('conflict-op', 'accept');
    });

    const state = useContentStore.getState();
    const op = state.optimisticOps.find(o => o.id === 'conflict-op');
    expect(op?.status).toBe('success');
  });
});