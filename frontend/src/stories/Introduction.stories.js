export default {
  title: 'Documentation/Introduction',
  parameters: {
    docs: {
      page: () => `
# Fastest Note App Component Library

Welcome to the **Fastest Note App** component documentation! This design system is built for high-performance note-taking with sub-200ms interaction targets.

## 🚀 Performance First

Every component is designed with performance in mind:

- **<100ms UI interactions** - All user interactions complete in under 100ms  
- **Optimistic updates** - UI updates before API confirmation
- **Virtual scrolling** - Handle large lists efficiently
- **Smart memoization** - Prevent unnecessary re-renders
- **Lazy loading** - Components load only when needed

## 📚 Component Categories

### Authentication
- **LoginForm** - User login interface with validation
- **RegisterForm** - User registration with password strength
- **AuthContainer** - Authentication flow wrapper

### Editor  
- **NoteEditor** - Rich text editor with auto-save
- Real-time character count and size limits
- Supports up to 1MB content size

### Navigation
- **FolderTree** - Hierarchical folder structure (max 10 levels)
- Drag-and-drop support for organization  
- Inline editing and context menus

### Notes
- **NoteList** - Virtual scrolled list for 1000+ notes
- Full-text search and filtering
- Sorting by date, title, or relevance

### Sync
- **ConflictResolver** - Handle offline/online conflicts
- Side-by-side diff view for changes
- Automatic and manual resolution options

## 📋 Performance Standards

- Component render time < 16ms (60fps)
- API response time < 200ms  
- UI interaction response < 100ms
- Supports up to 1000 items per folder

## 🎯 Getting Started

Browse the component categories in the sidebar to explore:
- Interactive examples
- Props documentation  
- Performance benchmarks
- Usage guidelines

*This documentation stays in sync with the actual component implementations.*
      `,
    },
  },
};

// Empty story to satisfy Storybook requirements
export const Page = () => null;