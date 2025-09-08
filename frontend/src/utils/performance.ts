/**
 * Performance optimization utilities
 * Tools for monitoring, optimizing, and measuring app performance
 */

// Performance monitoring
export class PerformanceMonitor {
  private static metrics: Map<string, number[]> = new Map();
  private static observers: Map<string, PerformanceObserver> = new Map();

  // Start performance measurement
  static startMeasurement(name: string): void {
    performance.mark(`${name}-start`);
  }

  // End performance measurement  
  static endMeasurement(name: string): number {
    performance.mark(`${name}-end`);
    performance.measure(name, `${name}-start`, `${name}-end`);
    
    const measure = performance.getEntriesByName(name, 'measure')[0] as PerformanceMeasure;
    const duration = measure.duration;
    
    // Store metrics for analysis
    if (!this.metrics.has(name)) {
      this.metrics.set(name, []);
    }
    this.metrics.get(name)!.push(duration);
    
    // Clean up marks
    performance.clearMarks(`${name}-start`);
    performance.clearMarks(`${name}-end`);
    performance.clearMeasures(name);
    
    return duration;
  }

  // Get performance statistics
  static getStats(name: string): { 
    avg: number; 
    min: number; 
    max: number; 
    count: number; 
  } | null {
    const measurements = this.metrics.get(name);
    if (!measurements || measurements.length === 0) {
      return null;
    }

    const avg = measurements.reduce((a, b) => a + b, 0) / measurements.length;
    const min = Math.min(...measurements);
    const max = Math.max(...measurements);
    
    return { avg, min, max, count: measurements.length };
  }

  // Monitor Core Web Vitals
  static observeWebVitals(): void {
    if (typeof window === 'undefined') return;

    // Largest Contentful Paint
    this.observeEntry('largest-contentful-paint', (entries) => {
      const entry = entries[entries.length - 1];
      console.log('LCP:', entry.startTime);
    });

    // First Input Delay
    this.observeEntry('first-input', (entries) => {
      const entry = entries[0];
      console.log('FID:', entry.processingStart - entry.startTime);
    });

    // Cumulative Layout Shift
    this.observeEntry('layout-shift', (entries) => {
      let clsScore = 0;
      entries.forEach((entry: any) => {
        if (!entry.hadRecentInput) {
          clsScore += entry.value;
        }
      });
      console.log('CLS:', clsScore);
    });
  }

  private static observeEntry(type: string, callback: (entries: PerformanceEntry[]) => void): void {
    if ('PerformanceObserver' in window) {
      try {
        const observer = new PerformanceObserver((list) => {
          callback(list.getEntries());
        });
        observer.observe({ type, buffered: true });
        this.observers.set(type, observer);
      } catch (e) {
        // Type not supported
      }
    }
  }

  // Report performance metrics
  static report(): void {
    if (process.env.NODE_ENV === 'development') {
      console.group('Performance Metrics');
      this.metrics.forEach((measurements, name) => {
        const stats = this.getStats(name);
        if (stats) {
          console.log(`${name}:`, {
            average: `${stats.avg.toFixed(2)}ms`,
            min: `${stats.min.toFixed(2)}ms`,
            max: `${stats.max.toFixed(2)}ms`,
            samples: stats.count,
          });
        }
      });
      console.groupEnd();
    }
  }

  // Clear all metrics
  static clear(): void {
    this.metrics.clear();
    this.observers.forEach(observer => observer.disconnect());
    this.observers.clear();
  }
}

// Higher-order component for measuring component render time
export const withPerformanceTracking = <P extends object>(
  Component: React.ComponentType<P>,
  componentName?: string
) => {
  const name = componentName || Component.displayName || Component.name || 'Component';
  
  return React.memo(React.forwardRef<any, P>((props, ref) => {
    React.useEffect(() => {
      PerformanceMonitor.startMeasurement(`render-${name}`);
      return () => {
        PerformanceMonitor.endMeasurement(`render-${name}`);
      };
    });

    return <Component {...props} ref={ref} />;
  }));
};

// Hook for performance measurement
export const usePerformanceTracker = (name: string) => {
  const startTime = React.useRef<number>();

  const start = React.useCallback(() => {
    startTime.current = performance.now();
  }, []);

  const end = React.useCallback(() => {
    if (startTime.current) {
      const duration = performance.now() - startTime.current;
      PerformanceMonitor.metrics.set(name, [
        ...(PerformanceMonitor.metrics.get(name) || []),
        duration
      ]);
      return duration;
    }
    return 0;
  }, [name]);

  return { start, end };
};

// Memory usage monitoring
export const memoryUtils = {
  // Get current memory usage
  getUsage(): MemoryInfo | null {
    if ('memory' in performance) {
      return (performance as any).memory;
    }
    return null;
  },

  // Format bytes to human readable
  formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  },

  // Log memory usage
  logUsage(): void {
    const usage = this.getUsage();
    if (usage) {
      console.log('Memory Usage:', {
        used: this.formatBytes(usage.usedJSHeapSize),
        total: this.formatBytes(usage.totalJSHeapSize),
        limit: this.formatBytes(usage.jsHeapSizeLimit),
      });
    }
  },

  // Monitor memory leaks
  startMemoryMonitoring(interval = 10000): NodeJS.Timer {
    return setInterval(() => {
      const usage = this.getUsage();
      if (usage) {
        const usedMB = usage.usedJSHeapSize / 1024 / 1024;
        if (usedMB > 50) { // Warn if using more than 50MB
          console.warn(`High memory usage: ${this.formatBytes(usage.usedJSHeapSize)}`);
        }
      }
    }, interval);
  },
};

// Network performance utilities
export const networkUtils = {
  // Get connection information
  getConnectionInfo(): NetworkInformation | null {
    return (navigator as any).connection || null;
  },

  // Check if connection is fast
  isFastConnection(): boolean {
    const connection = this.getConnectionInfo();
    if (!connection) return true; // Assume fast if unknown
    
    return connection.effectiveType === '4g' || 
           connection.downlink > 2; // > 2 Mbps
  },

  // Adapt based on network conditions
  adaptToNetwork<T>(fastOption: T, slowOption: T): T {
    return this.isFastConnection() ? fastOption : slowOption;
  },

  // Preload resources on fast connections
  conditionalPreload(urls: string[]): void {
    if (this.isFastConnection()) {
      urls.forEach(url => {
        const link = document.createElement('link');
        link.rel = 'prefetch';
        link.href = url;
        document.head.appendChild(link);
      });
    }
  },
};

// Debounce and throttle utilities
export const debounce = <T extends (...args: any[]) => any>(
  func: T,
  wait: number,
  immediate = false
): ((...args: Parameters<T>) => void) => {
  let timeout: NodeJS.Timeout | null = null;

  return function executedFunction(...args: Parameters<T>) {
    const later = () => {
      timeout = null;
      if (!immediate) func(...args);
    };

    const callNow = immediate && !timeout;

    if (timeout) clearTimeout(timeout);
    timeout = setTimeout(later, wait);

    if (callNow) func(...args);
  };
};

export const throttle = <T extends (...args: any[]) => any>(
  func: T,
  limit: number
): ((...args: Parameters<T>) => void) => {
  let inThrottle: boolean;

  return function executedFunction(...args: Parameters<T>) {
    if (!inThrottle) {
      func.apply(this, args);
      inThrottle = true;
      setTimeout(() => (inThrottle = false), limit);
    }
  };
};

// Image optimization utilities
export const imageUtils = {
  // Create optimized image loader
  createLoader: (src: string, quality = 75): string => {
    if (src.startsWith('/')) {
      // Local images - use Next.js optimization
      return `/_next/image?url=${encodeURIComponent(src)}&w=1200&q=${quality}`;
    }
    return src; // External images
  },

  // Lazy load images with Intersection Observer
  lazyLoad: (img: HTMLImageElement, src: string): void => {
    const observer = new IntersectionObserver((entries) => {
      entries.forEach(entry => {
        if (entry.isIntersecting) {
          img.src = src;
          img.classList.remove('lazy');
          observer.unobserve(img);
        }
      });
    });

    observer.observe(img);
  },

  // Create responsive image sources
  createSrcSet: (baseSrc: string, widths: number[]): string => {
    return widths
      .map(width => `${imageUtils.createLoader(baseSrc)} ${width}w`)
      .join(', ');
  },
};

// Bundle analysis utilities
export const bundleUtils = {
  // Analyze loaded chunks
  analyzeChunks: (): void => {
    if (process.env.NODE_ENV === 'development') {
      const scripts = Array.from(document.scripts);
      const chunks = scripts.filter(script => 
        script.src.includes('/_next/static/chunks/')
      );
      
      console.log('Loaded chunks:', chunks.length);
      chunks.forEach(chunk => {
        console.log(chunk.src.split('/').pop());
      });
    }
  },

  // Calculate bundle size impact
  measureBundleImpact: async (importFunc: () => Promise<any>): Promise<number> => {
    const before = performance.now();
    await importFunc();
    const after = performance.now();
    return after - before;
  },
};

// Error tracking for performance issues
export const errorTracking = {
  // Track performance-related errors
  trackError: (error: Error, context: string): void => {
    if (process.env.NODE_ENV === 'production') {
      // Send to error tracking service (e.g., Sentry)
      console.error(`Performance Error [${context}]:`, error);
    }
  },

  // Track slow operations
  trackSlowOperation: (operationName: string, duration: number, threshold = 1000): void => {
    if (duration > threshold) {
      console.warn(`Slow operation detected: ${operationName} took ${duration.toFixed(2)}ms`);
      
      if (process.env.NODE_ENV === 'production') {
        // Send to monitoring service
      }
    }
  },
};

// Initialize performance monitoring
export const initPerformanceMonitoring = (): void => {
  if (typeof window !== 'undefined') {
    // Start Web Vitals monitoring
    PerformanceMonitor.observeWebVitals();
    
    // Report metrics periodically in development
    if (process.env.NODE_ENV === 'development') {
      setInterval(() => PerformanceMonitor.report(), 30000);
    }
    
    // Monitor memory usage
    memoryUtils.startMemoryMonitoring();
    
    // Analyze chunks on load
    window.addEventListener('load', bundleUtils.analyzeChunks);
  }
};

// React hook for Web Vitals
export const useWebVitals = () => {
  const [vitals, setVitals] = React.useState<{
    lcp?: number;
    fid?: number;
    cls?: number;
  }>({});

  React.useEffect(() => {
    // This would integrate with web-vitals library
    // For now, we'll simulate the monitoring
    const updateVitals = (metric: string, value: number) => {
      setVitals(prev => ({ ...prev, [metric]: value }));
    };

    // Placeholder for actual web-vitals integration
    return () => {
      // Cleanup
    };
  }, []);

  return vitals;
};