import type {
  User,
  LoginRequest,
  CreateUserRequest,
  AuthResponse,
  RefreshTokenRequest,
  RefreshTokenResponse,
} from '@/types/auth';
import type {
  Folder,
  CreateFolderRequest,
  UpdateFolderRequest,
  MoveFolderRequest,
  FolderListResponse,
} from '@/types/folder';
import type {
  Note,
  CreateNoteRequest,
  UpdateNoteRequest,
  MoveNoteRequest,
  NoteListRequest,
  NoteListResponse,
} from '@/types/note';

class ApiError extends Error {
  constructor(
    message: string,
    public status: number,
    public data?: any
  ) {
    super(message);
    this.name = 'ApiError';
  }
}

class ApiClient {
  private baseUrl: string;
  private accessToken: string | null = null;
  private refreshToken: string | null = null;

  constructor(baseUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001') {
    this.baseUrl = baseUrl;
    
    if (typeof window !== 'undefined') {
      this.accessToken = localStorage.getItem('access_token');
      this.refreshToken = localStorage.getItem('refresh_token');
    }
  }

  private async request<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const url = `${this.baseUrl}${endpoint}`;
    const config: RequestInit = {
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
      },
      ...options,
    };

    if (this.accessToken) {
      config.headers = {
        ...config.headers,
        Authorization: `Bearer ${this.accessToken}`,
      };
    }

    try {
      const response = await fetch(url, config);

      if (!response.ok) {
        let errorData;
        try {
          errorData = await response.json();
        } catch {
          errorData = { error: response.statusText };
        }

        if (response.status === 401 && this.refreshToken) {
          try {
            await this.refreshAccessToken();
            config.headers = {
              ...config.headers,
              Authorization: `Bearer ${this.accessToken}`,
            };
            const retryResponse = await fetch(url, config);
            if (retryResponse.ok) {
              return await retryResponse.json();
            }
          } catch (refreshError) {
            this.clearTokens();
            throw new ApiError('Authentication failed', 401, errorData);
          }
        }

        throw new ApiError(
          errorData.error || `HTTP ${response.status}`,
          response.status,
          errorData
        );
      }

      if (response.status === 204) {
        return null as T;
      }

      return await response.json();
    } catch (error) {
      if (error instanceof ApiError) {
        throw error;
      }
      throw new ApiError('Network error', 0, error);
    }
  }

  private async refreshAccessToken(): Promise<void> {
    if (!this.refreshToken) {
      throw new Error('No refresh token available');
    }

    const response = await this.request<RefreshTokenResponse>('/auth/refresh', {
      method: 'POST',
      body: JSON.stringify({ refresh_token: this.refreshToken }),
    });

    this.setTokens(response.access_token, response.refresh_token);
  }

  private setTokens(accessToken: string, refreshToken: string): void {
    this.accessToken = accessToken;
    this.refreshToken = refreshToken;

    if (typeof window !== 'undefined') {
      localStorage.setItem('access_token', accessToken);
      localStorage.setItem('refresh_token', refreshToken);
    }
  }

  private clearTokens(): void {
    this.accessToken = null;
    this.refreshToken = null;

    if (typeof window !== 'undefined') {
      localStorage.removeItem('access_token');
      localStorage.removeItem('refresh_token');
    }
  }

  public isAuthenticated(): boolean {
    return !!this.accessToken;
  }

  public setAuthToken(accessToken: string, refreshToken: string): void {
    this.setTokens(accessToken, refreshToken);
  }

  public clearAuth(): void {
    this.clearTokens();
  }

  public auth = {
    login: async (data: LoginRequest): Promise<AuthResponse> => {
      const response = await this.request<AuthResponse>('/auth/login', {
        method: 'POST',
        body: JSON.stringify(data),
      });
      this.setTokens(response.access_token, response.refresh_token);
      return response;
    },

    register: async (data: CreateUserRequest): Promise<AuthResponse> => {
      const response = await this.request<AuthResponse>('/auth/register', {
        method: 'POST',
        body: JSON.stringify(data),
      });
      this.setTokens(response.access_token, response.refresh_token);
      return response;
    },

    refresh: async (data: RefreshTokenRequest): Promise<RefreshTokenResponse> => {
      const response = await this.request<RefreshTokenResponse>('/auth/refresh', {
        method: 'POST',
        body: JSON.stringify(data),
      });
      this.setTokens(response.access_token, response.refresh_token);
      return response;
    },

    logout: async (): Promise<void> => {
      try {
        await this.request('/auth/logout', {
          method: 'POST',
        });
      } finally {
        this.clearTokens();
      }
    },

    me: async (): Promise<User> => {
      return this.request<User>('/auth/me');
    },
  };

  public folders = {
    list: async (): Promise<Folder[]> => {
      const response = await this.request<FolderListResponse>('/folders');
      return response.folders || response as any;
    },

    get: async (id: string): Promise<Folder> => {
      return this.request<Folder>(`/folders/${id}`);
    },

    create: async (data: CreateFolderRequest): Promise<Folder> => {
      return this.request<Folder>('/folders', {
        method: 'POST',
        body: JSON.stringify(data),
      });
    },

    update: async (id: string, data: UpdateFolderRequest): Promise<Folder> => {
      return this.request<Folder>(`/folders/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
      });
    },

    delete: async (id: string): Promise<void> => {
      return this.request<void>(`/folders/${id}`, {
        method: 'DELETE',
      });
    },

    move: async (id: string, data: MoveFolderRequest): Promise<Folder> => {
      return this.request<Folder>(`/folders/${id}/move`, {
        method: 'POST',
        body: JSON.stringify(data),
      });
    },
  };

  public notes = {
    list: async (params: NoteListRequest = {}): Promise<Note[]> => {
      const searchParams = new URLSearchParams();
      
      if (params.folder_id) searchParams.append('folder_id', params.folder_id);
      if (params.search) searchParams.append('search', params.search);
      if (params.limit) searchParams.append('limit', params.limit.toString());
      if (params.offset) searchParams.append('offset', params.offset.toString());
      if (params.sort_by) searchParams.append('sort_by', params.sort_by);
      if (params.sort_order) searchParams.append('sort_order', params.sort_order);

      const query = searchParams.toString();
      const endpoint = `/notes${query ? `?${query}` : ''}`;
      
      const response = await this.request<NoteListResponse>(endpoint);
      return response.notes || response as any;
    },

    get: async (id: string): Promise<Note> => {
      return this.request<Note>(`/notes/${id}`);
    },

    create: async (data: CreateNoteRequest): Promise<Note> => {
      return this.request<Note>('/notes', {
        method: 'POST',
        body: JSON.stringify(data),
      });
    },

    update: async (id: string, data: UpdateNoteRequest): Promise<Note> => {
      return this.request<Note>(`/notes/${id}`, {
        method: 'PUT',
        body: JSON.stringify(data),
      });
    },

    delete: async (id: string): Promise<void> => {
      return this.request<void>(`/notes/${id}`, {
        method: 'DELETE',
      });
    },

    move: async (id: string, data: MoveNoteRequest): Promise<Note> => {
      return this.request<Note>(`/notes/${id}/move`, {
        method: 'POST',
        body: JSON.stringify(data),
      });
    },

    search: async (query: string, options: { limit?: number } = {}): Promise<Note[]> => {
      const searchParams = new URLSearchParams();
      searchParams.append('search', query);
      if (options.limit) searchParams.append('limit', options.limit.toString());

      const endpoint = `/notes/search?${searchParams.toString()}`;
      const response = await this.request<NoteListResponse>(endpoint);
      return response.notes || response as any;
    },
  };
}

export const api = new ApiClient();
export { ApiError };
export type { ApiClient };