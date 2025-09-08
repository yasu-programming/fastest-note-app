import { lazy, ComponentType } from 'react';

/**
 * Dynamic import utilities for code splitting
 * These functions create lazily loaded components with proper loading states
 */

// Higher-order function for creating lazy components with loading states
export const createLazyComponent = <T extends ComponentType<any>>(
  importFunc: () => Promise<{ default: T }>,
  fallback?: React.ComponentType
) => {
  return lazy(importFunc);
};

// Pre-configured lazy components for major app sections
export const LazyComponents = {
  // Authentication
  AuthContainer: createLazyComponent(
    () => import('@/components/Auth/AuthContainer')
  ),
  
  LoginForm: createLazyComponent(
    () => import('@/components/Auth/LoginForm')
  ),
  
  RegisterForm: createLazyComponent(
    () => import('@/components/Auth/RegisterForm')
  ),

  // Editor components
  NoteEditor: createLazyComponent(
    () => import('@/components/Editor/NoteEditor')
  ),

  // Navigation components  
  FolderTree: createLazyComponent(
    () => import('@/components/Navigation/FolderTree')
  ),

  // Notes components
  NoteList: createLazyComponent(
    () => import('@/components/Notes/NoteList')
  ),

  // Sync components
  ConflictResolver: createLazyComponent(
    () => import('@/components/Sync/ConflictResolver').then(mod => ({
      default: mod.ConflictResolver
    }))
  ),

  SyncStatus: createLazyComponent(
    () => import('@/components/Sync/ConflictResolver').then(mod => ({
      default: mod.SyncStatus
    }))
  ),

  // Settings components (future)
  SettingsPanel: createLazyComponent(
    () => import('@/components/Settings/SettingsPanel').catch(() => 
      import('./fallback-components').then(mod => ({ default: mod.SettingsFallback }))
    )
  ),

  // Dashboard components (future)
  Dashboard: createLazyComponent(
    () => import('@/components/Dashboard/Dashboard').catch(() =>
      import('./fallback-components').then(mod => ({ default: mod.DashboardFallback }))
    )
  ),
};

// Preload functions for critical components
export const preloadComponents = {
  auth: () => {
    import('@/components/Auth/AuthContainer');
    import('@/components/Auth/LoginForm');
    import('@/components/Auth/RegisterForm');
  },

  editor: () => {
    import('@/components/Editor/NoteEditor');
    import('@/components/Navigation/FolderTree');
    import('@/components/Notes/NoteList');
  },

  sync: () => {
    import('@/components/Sync/ConflictResolver');
  },

  all: () => {
    preloadComponents.auth();
    preloadComponents.editor();
    preloadComponents.sync();
  },
};

// Route-based code splitting
export const routeComponents = {
  '/auth': () => import('@/pages/auth'),
  '/dashboard': () => import('@/pages/dashboard'),
  '/note/[id]': () => import('@/pages/note/[id]'),
  '/folder/[id]': () => import('@/pages/folder/[id]'),
  '/settings': () => import('@/pages/settings'),
  '/search': () => import('@/pages/search'),
};

// Service worker for route prefetching
export const prefetchRoute = (route: keyof typeof routeComponents) => {
  if (typeof window !== 'undefined' && 'serviceWorker' in navigator) {
    // Use Intersection Observer to prefetch when links come into view
    const observer = new IntersectionObserver((entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          routeComponents[route]();
          observer.unobserve(entry.target);
        }
      });
    });

    // Find all links to this route and observe them
    const links = document.querySelectorAll(`a[href*="${route}"]`);
    links.forEach((link) => observer.observe(link));
  }
};

// Bundle splitting utilities
export const bundleUtils = {
  // Check if component is loaded
  isLoaded: (componentName: keyof typeof LazyComponents): boolean => {
    return !!LazyComponents[componentName]._payload;
  },

  // Preload component on user interaction
  preloadOnInteraction: (
    componentName: keyof typeof LazyComponents,
    element?: HTMLElement
  ) => {
    if (typeof window !== 'undefined') {
      const preload = () => LazyComponents[componentName];
      
      if (element) {
        element.addEventListener('mouseenter', preload, { once: true });
        element.addEventListener('focus', preload, { once: true });
      } else {
        // Preload on any user interaction
        const events = ['mousedown', 'touchstart', 'keydown'];
        const cleanup = () => {
          events.forEach(event => 
            document.removeEventListener(event, preload, { once: true })
          );
        };
        
        events.forEach(event => 
          document.addEventListener(event, () => {
            preload();
            cleanup();
          }, { once: true })
        );
      }
    }
  },

  // Preload based on network conditions
  preloadOnGoodConnection: () => {
    if (typeof window !== 'undefined' && 'connection' in navigator) {
      const connection = (navigator as any).connection;
      
      // Only preload on good connections (4g, wifi)
      if (connection && (connection.effectiveType === '4g' || connection.type === 'wifi')) {
        preloadComponents.all();
      }
    } else {
      // Fallback: preload after initial load
      setTimeout(() => preloadComponents.all(), 2000);
    }
  },
};

// Performance monitoring
export const performanceUtils = {
  // Measure component load time
  measureLoadTime: async (
    componentName: keyof typeof LazyComponents
  ): Promise<number> => {
    const start = performance.now();
    await LazyComponents[componentName];
    const end = performance.now();
    return end - start;
  },

  // Log bundle sizes in development
  logBundleSizes: () => {
    if (process.env.NODE_ENV === 'development') {
      console.group('Bundle Sizes');
      Object.keys(LazyComponents).forEach(async (componentName) => {
        const loadTime = await performanceUtils.measureLoadTime(
          componentName as keyof typeof LazyComponents
        );
        console.log(`${componentName}: ${loadTime.toFixed(2)}ms`);
      });
      console.groupEnd();
    }
  },
};