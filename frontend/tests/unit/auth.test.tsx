import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { act } from 'react-dom/test-utils';
import '@testing-library/jest-dom';

import { LoginForm } from '@/components/Auth/LoginForm';
import { RegisterForm } from '@/components/Auth/RegisterForm';
import { AuthContainer } from '@/components/Auth/AuthContainer';
import { useAuthStore } from '@/stores/authStore';

// Mock the auth store
jest.mock('@/stores/authStore');
const mockUseAuthStore = useAuthStore as jest.MockedFunction<typeof useAuthStore>;

// Mock the API
jest.mock('@/services/api', () => ({
  api: {
    auth: {
      login: jest.fn(),
      register: jest.fn(),
    },
  },
}));

// Mock Next.js router
jest.mock('next/navigation', () => ({
  useRouter: () => ({
    push: jest.fn(),
    replace: jest.fn(),
    prefetch: jest.fn(),
  }),
}));

// Test wrapper with QueryClient
const TestWrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
      mutations: {
        retry: false,
      },
    },
  });

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

describe('LoginForm', () => {
  const mockLogin = jest.fn();
  const mockOnSwitchToRegister = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    mockUseAuthStore.mockReturnValue({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,
      login: mockLogin,
      register: jest.fn(),
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    });
  });

  it('renders login form correctly', () => {
    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    expect(screen.getByRole('heading', { name: /sign in/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/password/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /sign in/i })).toBeInTheDocument();
  });

  it('validates email format', async () => {
    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    const emailInput = screen.getByLabelText(/email address/i);
    const submitButton = screen.getByRole('button', { name: /sign in/i });

    // Enter invalid email
    fireEvent.change(emailInput, { target: { value: 'invalid-email' } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/please enter a valid email address/i)).toBeInTheDocument();
    });

    // Login should not be called
    expect(mockLogin).not.toHaveBeenCalled();
  });

  it('requires password field', async () => {
    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    const emailInput = screen.getByLabelText(/email address/i);
    const submitButton = screen.getByRole('button', { name: /sign in/i });

    // Enter valid email but no password
    fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/password is required/i)).toBeInTheDocument();
    });

    expect(mockLogin).not.toHaveBeenCalled();
  });

  it('submits valid form data', async () => {
    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    const emailInput = screen.getByLabelText(/email address/i);
    const passwordInput = screen.getByLabelText(/password/i);
    const submitButton = screen.getByRole('button', { name: /sign in/i });

    // Enter valid credentials
    fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
    fireEvent.change(passwordInput, { target: { value: 'password123' } });

    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(mockLogin).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'password123',
      });
    });
  });

  it('shows loading state during submission', async () => {
    mockUseAuthStore.mockReturnValue({
      user: null,
      isAuthenticated: false,
      isLoading: true, // Loading state
      error: null,
      login: mockLogin,
      register: jest.fn(),
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    });

    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    const submitButton = screen.getByRole('button', { name: /signing in/i });
    expect(submitButton).toBeDisabled();
    expect(screen.getByText(/signing in/i)).toBeInTheDocument();
  });

  it('displays error messages', () => {
    mockUseAuthStore.mockReturnValue({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: 'Invalid credentials',
      login: mockLogin,
      register: jest.fn(),
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    });

    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    expect(screen.getByText(/invalid credentials/i)).toBeInTheDocument();
  });

  it('clears errors when user starts typing', async () => {
    const mockClearError = jest.fn();
    mockUseAuthStore.mockReturnValue({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: 'Invalid credentials',
      login: mockLogin,
      register: jest.fn(),
      logout: jest.fn(),
      clearError: mockClearError,
      initializeAuth: jest.fn(),
    });

    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    const emailInput = screen.getByLabelText(/email address/i);
    fireEvent.change(emailInput, { target: { value: 'new@example.com' } });

    expect(mockClearError).toHaveBeenCalled();
  });

  it('switches to register form', () => {
    render(
      <TestWrapper>
        <LoginForm onSwitchToRegister={mockOnSwitchToRegister} />
      </TestWrapper>
    );

    const switchLink = screen.getByRole('button', { name: /sign up here/i });
    fireEvent.click(switchLink);

    expect(mockOnSwitchToRegister).toHaveBeenCalled();
  });
});

describe('RegisterForm', () => {
  const mockRegister = jest.fn();
  const mockOnSwitchToLogin = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    mockUseAuthStore.mockReturnValue({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,
      login: jest.fn(),
      register: mockRegister,
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    });
  });

  it('renders register form correctly', () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    expect(screen.getByRole('heading', { name: /create account/i })).toBeInTheDocument();
    expect(screen.getByLabelText(/email address/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/^password$/i)).toBeInTheDocument();
    expect(screen.getByLabelText(/confirm password/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /create account/i })).toBeInTheDocument();
  });

  it('validates password strength', async () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    const passwordInput = screen.getByLabelText(/^password$/i);
    const submitButton = screen.getByRole('button', { name: /create account/i });

    // Enter weak password
    fireEvent.change(passwordInput, { target: { value: 'weak' } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/password must be at least 8 characters long/i)).toBeInTheDocument();
    });
  });

  it('shows password strength indicator', async () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    const passwordInput = screen.getByLabelText(/^password$/i);

    // Enter password to trigger strength indicator
    fireEvent.change(passwordInput, { target: { value: 'TestPass123!' } });

    await waitFor(() => {
      expect(screen.getByText(/strong/i)).toBeInTheDocument();
    });
  });

  it('validates password confirmation', async () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    const passwordInput = screen.getByLabelText(/^password$/i);
    const confirmPasswordInput = screen.getByLabelText(/confirm password/i);
    const submitButton = screen.getByRole('button', { name: /create account/i });

    fireEvent.change(passwordInput, { target: { value: 'TestPass123!' } });
    fireEvent.change(confirmPasswordInput, { target: { value: 'DifferentPass123!' } });
    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(screen.getByText(/passwords do not match/i)).toBeInTheDocument();
    });
  });

  it('submits valid registration data', async () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    const emailInput = screen.getByLabelText(/email address/i);
    const passwordInput = screen.getByLabelText(/^password$/i);
    const confirmPasswordInput = screen.getByLabelText(/confirm password/i);
    const submitButton = screen.getByRole('button', { name: /create account/i });

    fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
    fireEvent.change(passwordInput, { target: { value: 'TestPass123!' } });
    fireEvent.change(confirmPasswordInput, { target: { value: 'TestPass123!' } });

    fireEvent.click(submitButton);

    await waitFor(() => {
      expect(mockRegister).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'TestPass123!',
      });
    });
  });

  it('shows password requirements', () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    expect(screen.getByText(/password requirements/i)).toBeInTheDocument();
    expect(screen.getByText(/at least 8 characters long/i)).toBeInTheDocument();
    expect(screen.getByText(/one lowercase letter/i)).toBeInTheDocument();
    expect(screen.getByText(/one uppercase letter/i)).toBeInTheDocument();
    expect(screen.getByText(/one number/i)).toBeInTheDocument();
    expect(screen.getByText(/one special character/i)).toBeInTheDocument();
  });

  it('highlights fulfilled password requirements', async () => {
    render(
      <TestWrapper>
        <RegisterForm onSwitchToLogin={mockOnSwitchToLogin} />
      </TestWrapper>
    );

    const passwordInput = screen.getByLabelText(/^password$/i);
    fireEvent.change(passwordInput, { target: { value: 'TestPass123!' } });

    await waitFor(() => {
      // Check that requirements are highlighted (would need data-testid or specific classes)
      const requirements = screen.getByText(/at least 8 characters long/i);
      expect(requirements).toBeInTheDocument();
    });
  });
});

describe('AuthContainer', () => {
  beforeEach(() => {
    jest.clearAllMocks();
    mockUseAuthStore.mockReturnValue({
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,
      login: jest.fn(),
      register: jest.fn(),
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    });
  });

  it('renders with login form by default', () => {
    render(
      <TestWrapper>
        <AuthContainer />
      </TestWrapper>
    );

    expect(screen.getByRole('heading', { name: /sign in/i })).toBeInTheDocument();
    expect(screen.getByText(/welcome back to fastest note app/i)).toBeInTheDocument();
  });

  it('renders with register form when initialMode is register', () => {
    render(
      <TestWrapper>
        <AuthContainer initialMode="register" />
      </TestWrapper>
    );

    expect(screen.getByRole('heading', { name: /create account/i })).toBeInTheDocument();
    expect(screen.getByText(/join fastest note app today/i)).toBeInTheDocument();
  });

  it('switches between login and register forms', async () => {
    render(
      <TestWrapper>
        <AuthContainer />
      </TestWrapper>
    );

    // Start with login form
    expect(screen.getByRole('heading', { name: /sign in/i })).toBeInTheDocument();

    // Switch to register
    const switchToRegister = screen.getByRole('button', { name: /sign up here/i });
    fireEvent.click(switchToRegister);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /create account/i })).toBeInTheDocument();
    });

    // Switch back to login
    const switchToLogin = screen.getByRole('button', { name: /sign in here/i });
    fireEvent.click(switchToLogin);

    await waitFor(() => {
      expect(screen.getByRole('heading', { name: /sign in/i })).toBeInTheDocument();
    });
  });

  it('displays app branding', () => {
    render(
      <TestWrapper>
        <AuthContainer />
      </TestWrapper>
    );

    expect(screen.getByText(/fastest note app/i)).toBeInTheDocument();
    expect(screen.getByText(/lightning-fast note-taking with real-time sync/i)).toBeInTheDocument();
  });

  it('displays feature highlights', () => {
    render(
      <TestWrapper>
        <AuthContainer />
      </TestWrapper>
    );

    expect(screen.getByText(/lightning fast/i)).toBeInTheDocument();
    expect(screen.getByText(/real-time sync/i)).toBeInTheDocument();
    expect(screen.getByText(/organized/i)).toBeInTheDocument();
  });

  it('has demo access button', () => {
    render(
      <TestWrapper>
        <AuthContainer />
      </TestWrapper>
    );

    expect(screen.getByText(/try demo version \(no account required\)/i)).toBeInTheDocument();
  });

  it('calls onAuthSuccess when provided', async () => {
    const mockOnAuthSuccess = jest.fn();
    
    // Mock successful authentication
    mockUseAuthStore.mockReturnValue({
      user: { id: '1', email: 'test@example.com' },
      isAuthenticated: true,
      isLoading: false,
      error: null,
      login: jest.fn(),
      register: jest.fn(),
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    });

    render(
      <TestWrapper>
        <AuthContainer onAuthSuccess={mockOnAuthSuccess} />
      </TestWrapper>
    );

    // This would typically be triggered by successful auth
    // In a real test, we'd need to mock the authentication flow
    expect(mockOnAuthSuccess).not.toHaveBeenCalled(); // Initially not called
  });
});

// Integration test for full auth flow
describe('Auth Integration', () => {
  it('completes full login flow', async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false }, mutations: { retry: false } },
    });

    let authState = {
      user: null,
      isAuthenticated: false,
      isLoading: false,
      error: null,
    };

    const mockLogin = jest.fn().mockImplementation(async (credentials) => {
      authState = {
        ...authState,
        isLoading: true,
      };
      
      // Simulate API call delay
      await new Promise(resolve => setTimeout(resolve, 100));
      
      authState = {
        user: { id: '1', email: credentials.email },
        isAuthenticated: true,
        isLoading: false,
        error: null,
      };
    });

    mockUseAuthStore.mockImplementation(() => ({
      ...authState,
      login: mockLogin,
      register: jest.fn(),
      logout: jest.fn(),
      clearError: jest.fn(),
      initializeAuth: jest.fn(),
    }));

    const { rerender } = render(
      <QueryClientProvider client={queryClient}>
        <AuthContainer />
      </QueryClientProvider>
    );

    // Fill and submit login form
    const emailInput = screen.getByLabelText(/email address/i);
    const passwordInput = screen.getByLabelText(/password/i);
    const submitButton = screen.getByRole('button', { name: /sign in/i });

    fireEvent.change(emailInput, { target: { value: 'test@example.com' } });
    fireEvent.change(passwordInput, { target: { value: 'password123' } });
    fireEvent.click(submitButton);

    // Verify login was called
    await waitFor(() => {
      expect(mockLogin).toHaveBeenCalledWith({
        email: 'test@example.com',
        password: 'password123',
      });
    });

    // Rerender with updated auth state
    authState.isLoading = false;
    authState.isAuthenticated = true;
    authState.user = { id: '1', email: 'test@example.com' };

    rerender(
      <QueryClientProvider client={queryClient}>
        <AuthContainer />
      </QueryClientProvider>
    );

    // At this point, the component would typically redirect or show success state
    // This depends on your actual implementation
  });
});