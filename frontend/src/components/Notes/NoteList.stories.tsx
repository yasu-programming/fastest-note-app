import type { Meta, StoryObj } from '@storybook/react';
import { fn } from '@storybook/test';
import { NoteList } from './NoteList';
import type { Note } from '@/types/note';

const generateSampleNotes = (count: number): Note[] => {
  return Array.from({ length: count }, (_, i) => ({
    id: `note-${i + 1}`,
    title: `Note ${i + 1}`,
    content: `This is the content for note ${i + 1}. ${i % 3 === 0 ? 'This note has longer content to test different content lengths in the list view.' : ''}`,
    folder_id: i % 2 === 0 ? 'folder-1' : null,
    user_id: 'user-1',
    version: 1,
    created_at: new Date(2023, 11, i + 1, 10, 0, 0),
    updated_at: new Date(2023, 11, i + 1, 15, 30, 0),
  }));
};

const sampleNotes = generateSampleNotes(10);
const manyNotes = generateSampleNotes(1000);

const meta: Meta<typeof NoteList> = {
  title: 'Notes/NoteList',
  component: NoteList,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component: 'Virtualized list component for displaying notes with search, sort, and infinite scroll. Optimized to handle thousands of notes efficiently.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    folderId: {
      description: 'Filter notes by folder ID',
      control: { type: 'text' },
    },
    selectedNoteId: {
      description: 'ID of currently selected note',
      control: { type: 'text' },
    },
    searchQuery: {
      description: 'Search query to filter notes',
      control: { type: 'text' },
    },
    sortBy: {
      description: 'Field to sort notes by',
      control: { type: 'select' },
      options: ['updated_at', 'created_at', 'title'],
    },
    sortOrder: {
      description: 'Sort order',
      control: { type: 'select' },
      options: ['asc', 'desc'],
    },
    onNoteSelect: {
      description: 'Callback when a note is selected',
    },
    onNoteCreate: {
      description: 'Callback when a new note is created',
    },
    onNoteDelete: {
      description: 'Callback when a note is deleted',
    },
  },
  args: {
    onNoteSelect: fn(),
    onNoteCreate: fn(),
    onNoteDelete: fn(),
    sortBy: 'updated_at',
    sortOrder: 'desc',
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Default note list with sample notes.',
      },
    },
  },
};

export const WithSelectedNote: Story = {
  args: {
    selectedNoteId: 'note-3',
  },
  parameters: {
    docs: {
      description: {
        story: 'Note list with a selected note highlighted.',
      },
    },
  },
};

export const WithSearch: Story = {
  args: {
    searchQuery: 'longer content',
  },
  parameters: {
    docs: {
      description: {
        story: 'Note list filtered by search query.',
      },
    },
  },
};

export const SortedByTitle: Story = {
  args: {
    sortBy: 'title',
    sortOrder: 'asc',
  },
  parameters: {
    docs: {
      description: {
        story: 'Note list sorted alphabetically by title.',
      },
    },
  },
};

export const FilteredByFolder: Story = {
  args: {
    folderId: 'folder-1',
  },
  parameters: {
    docs: {
      description: {
        story: 'Note list filtered to show only notes in a specific folder.',
      },
    },
  },
};

export const VirtualizedPerformance: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Note list with many notes to demonstrate virtualization performance (1000+ notes).',
      },
    },
  },
};

export const EmptyState: Story = {
  args: {
    searchQuery: 'nonexistent',
  },
  parameters: {
    docs: {
      description: {
        story: 'Empty state when no notes match the current filters.',
      },
    },
  },
};