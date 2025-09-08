import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { api } from '@/services/api';
import type { User, AuthResponse } from '@/types/auth';

export interface AuthState {
  user: User | null;
  accessToken: string | null;
  refreshToken: string | null;
  isAuthenticated: boolean;
  isLoading: boolean;
  error: string | null;
}

export interface AuthActions {
  login: (credentials: { email: string; password: string }) => Promise<void>;
  register: (userData: { email: string; password: string }) => Promise<void>;
  logout: () => void;
  clearError: () => void;
  refreshAuth: () => Promise<void>;
  setLoading: (loading: boolean) => void;
  initializeAuth: () => Promise<void>;
}

export type AuthStore = AuthState & AuthActions;

const initialState: AuthState = {
  user: null,
  accessToken: null,
  refreshToken: null,
  isAuthenticated: false,
  isLoading: false,
  error: null,
};

export const useAuthStore = create<AuthStore>()(
  persist(
    (set, get) => ({
      ...initialState,

      login: async (credentials: { email: string; password: string }) => {
        try {
          set({ isLoading: true, error: null });

          const response: AuthResponse = await api.auth.login(credentials);
          
          api.setAuthToken(response.access_token, response.refresh_token);

          set({
            user: response.user,
            accessToken: response.access_token,
            refreshToken: response.refresh_token,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });
        } catch (error: any) {
          set({
            isLoading: false,
            error: error.message || 'Login failed',
            isAuthenticated: false,
          });
          throw error;
        }
      },

      register: async (userData: { email: string; password: string }) => {
        try {
          set({ isLoading: true, error: null });

          const response: AuthResponse = await api.auth.register(userData);
          
          api.setAuthToken(response.access_token, response.refresh_token);

          set({
            user: response.user,
            accessToken: response.access_token,
            refreshToken: response.refresh_token,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });
        } catch (error: any) {
          set({
            isLoading: false,
            error: error.message || 'Registration failed',
            isAuthenticated: false,
          });
          throw error;
        }
      },

      logout: () => {
        api.clearAuth();
        
        set({
          user: null,
          accessToken: null,
          refreshToken: null,
          isAuthenticated: false,
          error: null,
        });

        if (typeof window !== 'undefined') {
          window.location.href = '/auth';
        }
      },

      clearError: () => {
        set({ error: null });
      },

      refreshAuth: async () => {
        const { refreshToken } = get();
        
        if (!refreshToken) {
          get().logout();
          return;
        }

        try {
          set({ isLoading: true, error: null });

          const response = await api.auth.refresh({ refresh_token: refreshToken });
          
          api.setAuthToken(response.access_token, response.refresh_token);

          set({
            accessToken: response.access_token,
            refreshToken: response.refresh_token,
            isLoading: false,
            error: null,
          });
        } catch (error: any) {
          console.error('Token refresh failed:', error);
          set({
            isLoading: false,
            error: 'Session expired. Please login again.',
          });
          get().logout();
        }
      },

      setLoading: (loading: boolean) => {
        set({ isLoading: loading });
      },

      initializeAuth: async () => {
        const { accessToken, refreshToken } = get();

        if (!accessToken || !refreshToken) {
          return;
        }

        try {
          set({ isLoading: true });

          api.setAuthToken(accessToken, refreshToken);
          
          const user = await api.auth.me();

          set({
            user,
            isAuthenticated: true,
            isLoading: false,
            error: null,
          });
        } catch (error: any) {
          console.error('Auth initialization failed:', error);
          
          if (refreshToken) {
            try {
              await get().refreshAuth();
              const user = await api.auth.me();
              set({
                user,
                isAuthenticated: true,
                isLoading: false,
                error: null,
              });
            } catch (refreshError) {
              console.error('Refresh during initialization failed:', refreshError);
              get().logout();
            }
          } else {
            set({
              isLoading: false,
              error: null,
            });
            get().logout();
          }
        }
      },
    }),
    {
      name: 'auth-store',
      partialize: (state) => ({
        user: state.user,
        accessToken: state.accessToken,
        refreshToken: state.refreshToken,
        isAuthenticated: state.isAuthenticated,
      }),
      onRehydrateStorage: () => (state) => {
        if (state?.accessToken && state?.refreshToken) {
          api.setAuthToken(state.accessToken, state.refreshToken);
        }
      },
    }
  )
);

export const useAuth = () => {
  const {
    user,
    isAuthenticated,
    isLoading,
    error,
    login,
    register,
    logout,
    clearError,
    initializeAuth,
  } = useAuthStore();

  return {
    user,
    isAuthenticated,
    isLoading,
    error,
    login,
    register,
    logout,
    clearError,
    initializeAuth,
  };
};

if (typeof window !== 'undefined') {
  const checkTokenExpiry = () => {
    const state = useAuthStore.getState();
    if (state.isAuthenticated && state.accessToken) {
      try {
        const payload = JSON.parse(atob(state.accessToken.split('.')[1]));
        const now = Date.now() / 1000;
        
        if (payload.exp && payload.exp - now < 300) {
          state.refreshAuth();
        }
      } catch (error) {
        console.error('Error checking token expiry:', error);
      }
    }
  };

  setInterval(checkTokenExpiry, 60000);
}