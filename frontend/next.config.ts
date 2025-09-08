import type { NextConfig } from "next";
import { BundleAnalyzerPlugin } from 'webpack-bundle-analyzer';

const nextConfig: NextConfig = {
  // Enable experimental features for performance
  experimental: {
    // Server Components optimizations
    serverComponentsExternalPackages: ['@tanstack/react-query'],
    
    // Optimize CSS loading
    optimizeCss: true,
    
    // Enable SWC minification for better performance
    swcMinify: true,
    
    // Memory optimization
    memoryBasedWorkers: true,
  },

  // Compiler optimizations
  compiler: {
    // Remove console logs in production
    removeConsole: process.env.NODE_ENV === 'production' ? {
      exclude: ['error', 'warn']
    } : false,
    
    // Enable styled-components SSR
    styledComponents: true,
  },

  // Image optimization
  images: {
    // Optimize image loading
    formats: ['image/webp', 'image/avif'],
    deviceSizes: [640, 768, 1024, 1280, 1600],
    imageSizes: [16, 32, 48, 64, 96, 128, 256, 384],
    
    // Enable image optimization
    minimumCacheTTL: 31536000, // 1 year
    
    // Allow external image domains if needed
    domains: [],
    
    // Disable image optimization for development
    unoptimized: process.env.NODE_ENV === 'development',
  },

  // Performance optimizations
  poweredByHeader: false,
  compress: true,
  
  // Build optimization
  generateEtags: false,
  
  // Bundle optimization
  webpack: (config, { buildId, dev, isServer, defaultLoaders, nextRuntime, webpack }) => {
    // Optimize bundle splitting
    if (!dev && !isServer) {
      config.optimization = {
        ...config.optimization,
        splitChunks: {
          chunks: 'all',
          cacheGroups: {
            // Separate vendor chunks
            vendor: {
              test: /[\\/]node_modules[\\/]/,
              name: 'vendors',
              priority: 20,
              chunks: 'all',
              reuseExistingChunk: true,
            },
            
            // React and related libraries
            react: {
              test: /[\\/]node_modules[\\/](react|react-dom|react-router)[\\/]/,
              name: 'react',
              priority: 30,
              chunks: 'all',
              reuseExistingChunk: true,
            },
            
            // UI libraries
            ui: {
              test: /[\\/]node_modules[\\/](@headlessui|@heroicons|clsx|tailwindcss)[\\/]/,
              name: 'ui',
              priority: 25,
              chunks: 'all',
              reuseExistingChunk: true,
            },
            
            // Data fetching libraries
            api: {
              test: /[\\/]node_modules[\\/](@tanstack|axios|ky)[\\/]/,
              name: 'api',
              priority: 25,
              chunks: 'all',
              reuseExistingChunk: true,
            },
            
            // State management
            state: {
              test: /[\\/]node_modules[\\/](zustand|immer)[\\/]/,
              name: 'state',
              priority: 25,
              chunks: 'all',
              reuseExistingChunk: true,
            },
            
            // Utilities
            utils: {
              test: /[\\/]node_modules[\\/](date-fns|uuid|validator|zod)[\\/]/,
              name: 'utils',
              priority: 15,
              chunks: 'all',
              reuseExistingChunk: true,
            },
            
            // Common application code
            common: {
              name: 'common',
              minChunks: 2,
              priority: 10,
              chunks: 'all',
              reuseExistingChunk: true,
            },
          },
        },
      };

      // Tree shaking optimization
      config.optimization.usedExports = true;
      config.optimization.sideEffects = false;
      
      // Module concatenation
      config.optimization.concatenateModules = true;
    }

    // Bundle analyzer in production
    if (!dev && process.env.ANALYZE === 'true') {
      config.plugins.push(
        new BundleAnalyzerPlugin({
          analyzerMode: 'static',
          openAnalyzer: false,
          reportFilename: 'bundle-analyzer-report.html',
        })
      );
    }

    // Optimize imports
    config.resolve.alias = {
      ...config.resolve.alias,
      // Ensure single instance of React
      'react': require.resolve('react'),
      'react-dom': require.resolve('react-dom'),
    };

    // Module federation for micro-frontends (if needed in future)
    if (!dev && !isServer) {
      config.optimization.moduleIds = 'deterministic';
      config.optimization.chunkIds = 'deterministic';
    }

    return config;
  },

  // Output configuration
  output: process.env.NODE_ENV === 'production' ? 'standalone' : undefined,
  
  // Environment variables
  env: {
    CUSTOM_KEY: process.env.CUSTOM_KEY,
  },

  // Headers for performance
  async headers() {
    return [
      {
        source: '/:path*',
        headers: [
          {
            key: 'X-DNS-Prefetch-Control',
            value: 'on'
          },
          {
            key: 'X-XSS-Protection',
            value: '1; mode=block'
          },
          {
            key: 'X-Frame-Options',
            value: 'SAMEORIGIN'
          },
          {
            key: 'X-Content-Type-Options',
            value: 'nosniff'
          },
        ],
      },
      {
        source: '/api/:path*',
        headers: [
          {
            key: 'Cache-Control',
            value: 'public, max-age=0, must-revalidate',
          },
        ],
      },
      {
        source: '/_next/static/:path*',
        headers: [
          {
            key: 'Cache-Control',
            value: 'public, max-age=31536000, immutable',
          },
        ],
      },
    ];
  },

  // Rewrites for API routes
  async rewrites() {
    return [
      {
        source: '/api/:path*',
        destination: `${process.env.NEXT_PUBLIC_API_URL || 'http://localhost:3001'}/:path*`,
      },
    ];
  },

  // Redirects for SEO
  async redirects() {
    return [
      // Add redirects as needed
    ];
  },
};

export default nextConfig;
