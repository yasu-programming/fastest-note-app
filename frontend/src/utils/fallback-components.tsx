import React from 'react';

/**
 * Fallback components for lazy loading and error boundaries
 * These provide user-friendly loading and error states
 */

// Generic loading spinner component
export const LoadingSpinner: React.FC<{ size?: 'sm' | 'md' | 'lg' }> = ({ 
  size = 'md' 
}) => {
  const sizeClasses = {
    sm: 'w-4 h-4',
    md: 'w-8 h-8', 
    lg: 'w-12 h-12'
  };

  return (
    <div className="flex items-center justify-center p-4">
      <div className={`animate-spin rounded-full border-2 border-blue-500 border-t-transparent ${sizeClasses[size]}`} />
    </div>
  );
};

// Loading skeleton for different component types
export const LoadingSkeleton = {
  // Note editor skeleton
  Editor: () => (
    <div className="animate-pulse p-6 space-y-4">
      <div className="h-4 bg-gray-200 rounded w-1/4"></div>
      <div className="h-8 bg-gray-200 rounded w-full"></div>
      <div className="space-y-2">
        <div className="h-4 bg-gray-200 rounded w-full"></div>
        <div className="h-4 bg-gray-200 rounded w-3/4"></div>
        <div className="h-4 bg-gray-200 rounded w-5/6"></div>
      </div>
    </div>
  ),

  // Note list skeleton
  List: () => (
    <div className="animate-pulse space-y-4 p-4">
      {Array.from({ length: 5 }).map((_, i) => (
        <div key={i} className="border-b border-gray-200 pb-4">
          <div className="flex justify-between items-start mb-2">
            <div className="h-5 bg-gray-200 rounded w-1/3"></div>
            <div className="h-4 bg-gray-200 rounded w-16"></div>
          </div>
          <div className="space-y-1">
            <div className="h-4 bg-gray-200 rounded w-full"></div>
            <div className="h-4 bg-gray-200 rounded w-4/5"></div>
          </div>
        </div>
      ))}
    </div>
  ),

  // Folder tree skeleton
  Tree: () => (
    <div className="animate-pulse p-4 space-y-2">
      <div className="h-6 bg-gray-200 rounded w-1/2"></div>
      {Array.from({ length: 3 }).map((_, i) => (
        <div key={i} className="ml-4 space-y-1">
          <div className="h-4 bg-gray-200 rounded w-3/4"></div>
          <div className="ml-4 h-4 bg-gray-200 rounded w-1/2"></div>
        </div>
      ))}
    </div>
  ),

  // Auth form skeleton
  Auth: () => (
    <div className="animate-pulse max-w-md mx-auto bg-white rounded-lg shadow-md p-6 space-y-4">
      <div className="text-center space-y-2">
        <div className="h-8 bg-gray-200 rounded w-1/2 mx-auto"></div>
        <div className="h-4 bg-gray-200 rounded w-3/4 mx-auto"></div>
      </div>
      <div className="space-y-4">
        <div className="space-y-2">
          <div className="h-4 bg-gray-200 rounded w-1/4"></div>
          <div className="h-10 bg-gray-200 rounded w-full"></div>
        </div>
        <div className="space-y-2">
          <div className="h-4 bg-gray-200 rounded w-1/4"></div>
          <div className="h-10 bg-gray-200 rounded w-full"></div>
        </div>
        <div className="h-10 bg-gray-200 rounded w-full"></div>
      </div>
    </div>
  ),
};

// Error boundary fallback components
export const ErrorFallback: React.FC<{ 
  error?: Error;
  resetError?: () => void;
  componentName?: string;
}> = ({ error, resetError, componentName = 'Component' }) => (
  <div className="flex flex-col items-center justify-center p-8 bg-red-50 border border-red-200 rounded-lg">
    <div className="text-red-600 mb-4">
      <svg className="w-12 h-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path 
          strokeLinecap="round" 
          strokeLinejoin="round" 
          strokeWidth={2} 
          d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-2.5L13.732 4c-.77-.833-1.964-.833-2.732 0L3.732 16.5c-.77.833.192 2.5 1.732 2.5z" 
        />
      </svg>
    </div>
    
    <h3 className="text-lg font-semibold text-red-900 mb-2">
      {componentName} failed to load
    </h3>
    
    <p className="text-red-700 text-center mb-4 max-w-md">
      {error?.message || 'An unexpected error occurred while loading this component.'}
    </p>
    
    {resetError && (
      <button
        onClick={resetError}
        className="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700 transition-colors"
      >
        Try Again
      </button>
    )}
  </div>
);

// Network-aware fallbacks
export const NetworkErrorFallback: React.FC<{ onRetry?: () => void }> = ({ onRetry }) => (
  <div className="flex flex-col items-center justify-center p-8 bg-yellow-50 border border-yellow-200 rounded-lg">
    <div className="text-yellow-600 mb-4">
      <svg className="w-12 h-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path 
          strokeLinecap="round" 
          strokeLinejoin="round" 
          strokeWidth={2} 
          d="M12 8v4m0 4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" 
        />
      </svg>
    </div>
    
    <h3 className="text-lg font-semibold text-yellow-900 mb-2">
      Connection Issue
    </h3>
    
    <p className="text-yellow-700 text-center mb-4">
      Unable to load content. Please check your internet connection.
    </p>
    
    {onRetry && (
      <button
        onClick={onRetry}
        className="px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700 transition-colors"
      >
        Retry
      </button>
    )}
  </div>
);

// Placeholder components for future features
export const SettingsFallback: React.FC = () => (
  <div className="p-8 text-center">
    <div className="w-16 h-16 bg-gray-200 rounded-full mx-auto mb-4 flex items-center justify-center">
      <svg className="w-8 h-8 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
      </svg>
    </div>
    <h3 className="text-lg font-medium text-gray-900 mb-2">Settings Coming Soon</h3>
    <p className="text-gray-600">User settings and preferences will be available in a future update.</p>
  </div>
);

export const DashboardFallback: React.FC = () => (
  <div className="p-8 text-center">
    <div className="w-16 h-16 bg-gray-200 rounded-full mx-auto mb-4 flex items-center justify-center">
      <svg className="w-8 h-8 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
      </svg>
    </div>
    <h3 className="text-lg font-medium text-gray-900 mb-2">Analytics Dashboard</h3>
    <p className="text-gray-600">Advanced analytics and insights will be available in a future update.</p>
  </div>
);

// Offline fallback
export const OfflineFallback: React.FC = () => (
  <div className="flex flex-col items-center justify-center p-8 bg-gray-50 border border-gray-200 rounded-lg">
    <div className="text-gray-400 mb-4">
      <svg className="w-12 h-12" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18.364 5.636l-3.536 3.536m0 5.656l3.536 3.536M9.172 9.172L5.636 5.636m3.536 9.192L5.636 18.364M12 2.18l6.364 6.364a9 9 0 010 12.728L12 21.82l-6.364-6.364a9 9 0 010-12.728L12 2.18z" />
      </svg>
    </div>
    <h3 className="text-lg font-semibold text-gray-700 mb-2">You're Offline</h3>
    <p className="text-gray-600 text-center">
      Your changes are being saved locally and will sync when you're back online.
    </p>
  </div>
);

// Component wrapper for error boundary
export const withErrorBoundary = <P extends object>(
  Component: React.ComponentType<P>,
  fallback?: React.ComponentType<{ error?: Error; resetError?: () => void }>
) => {
  const WrappedComponent: React.FC<P> = (props) => {
    const [hasError, setHasError] = React.useState(false);
    const [error, setError] = React.useState<Error | null>(null);

    React.useEffect(() => {
      const errorHandler = (event: ErrorEvent) => {
        setError(new Error(event.message));
        setHasError(true);
      };

      window.addEventListener('error', errorHandler);
      return () => window.removeEventListener('error', errorHandler);
    }, []);

    const resetError = () => {
      setHasError(false);
      setError(null);
    };

    if (hasError) {
      const FallbackComponent = fallback || ErrorFallback;
      return <FallbackComponent error={error || undefined} resetError={resetError} />;
    }

    return <Component {...props} />;
  };

  WrappedComponent.displayName = `withErrorBoundary(${Component.displayName || Component.name})`;
  return WrappedComponent;
};