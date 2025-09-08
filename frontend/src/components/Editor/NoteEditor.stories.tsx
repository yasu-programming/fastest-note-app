import type { Meta, StoryObj } from '@storybook/react';
import { fn } from '@storybook/test';
import { NoteEditor } from './NoteEditor';
import type { Note } from '@/types/note';

const sampleNote: Note = {
  id: '123e4567-e89b-12d3-a456-426614174000',
  title: 'Sample Note',
  content: 'This is a sample note content with some text...',
  folder_id: '456e7890-e89b-12d3-a456-426614174001',
  user_id: '789e0123-e89b-12d3-a456-426614174002',
  version: 1,
  created_at: new Date('2023-12-07T10:30:00Z'),
  updated_at: new Date('2023-12-07T10:30:00Z'),
};

const meta: Meta<typeof NoteEditor> = {
  title: 'Editor/NoteEditor',
  component: NoteEditor,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component: 'Rich text editor component for creating and editing notes with auto-save functionality. Supports notes up to 1MB in size.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    note: {
      description: 'Existing note to edit',
      control: { type: 'object' },
    },
    folderId: {
      description: 'ID of the folder to save new note to',
      control: { type: 'text' },
    },
    placeholder: {
      description: 'Placeholder text for empty content area',
      control: { type: 'text' },
    },
    onSave: {
      description: 'Callback when note is saved',
    },
    onCancel: {
      description: 'Callback when editing is cancelled',
    },
  },
  args: {
    onSave: fn(),
    onCancel: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const NewNote: Story = {
  args: {
    folderId: '456e7890-e89b-12d3-a456-426614174001',
    placeholder: 'Start typing your new note...',
  },
  parameters: {
    docs: {
      description: {
        story: 'Editor for creating a new note from scratch.',
      },
    },
  },
};

export const EditingExistingNote: Story = {
  args: {
    note: sampleNote,
  },
  parameters: {
    docs: {
      description: {
        story: 'Editor loaded with an existing note for editing.',
      },
    },
  },
};

export const EmptyNote: Story = {
  args: {
    note: {
      ...sampleNote,
      title: '',
      content: '',
    },
  },
  parameters: {
    docs: {
      description: {
        story: 'Editor with an existing but empty note.',
      },
    },
  },
};

export const LongContent: Story = {
  args: {
    note: {
      ...sampleNote,
      title: 'Very Long Note',
      content: 'This is a very long note content. '.repeat(50) + '\n\nWith multiple paragraphs and lots of text to demonstrate how the editor handles larger content sizes.',
    },
  },
  parameters: {
    docs: {
      description: {
        story: 'Editor with a note containing long content to test scrolling and performance.',
      },
    },
  },
};