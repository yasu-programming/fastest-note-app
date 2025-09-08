import type { Meta, StoryObj } from '@storybook/react';
import { fn } from '@storybook/test';
import { ConflictResolver } from './ConflictResolver';

const meta: Meta<typeof ConflictResolver> = {
  title: 'Sync/ConflictResolver',
  component: ConflictResolver,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component: 'Conflict resolution UI for handling sync conflicts when working offline. Shows differences between local and remote changes and allows users to choose resolution strategy.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    isOpen: {
      description: 'Whether the conflict resolver modal is open',
      control: { type: 'boolean' },
    },
    onClose: {
      description: 'Callback when conflict resolver is closed',
    },
    onAllResolved: {
      description: 'Callback when all conflicts have been resolved',
    },
  },
  args: {
    onClose: fn(),
    onAllResolved: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Closed: Story = {
  args: {
    isOpen: false,
  },
  parameters: {
    docs: {
      description: {
        story: 'Conflict resolver in closed state.',
      },
    },
  },
};

export const SingleConflict: Story = {
  args: {
    isOpen: true,
  },
  parameters: {
    docs: {
      description: {
        story: 'Conflict resolver showing a single sync conflict that needs resolution.',
      },
    },
  },
};

export const MultipleConflicts: Story = {
  args: {
    isOpen: true,
  },
  parameters: {
    docs: {
      description: {
        story: 'Conflict resolver showing multiple sync conflicts with different resolution options.',
      },
    },
  },
};

export const NoteContentConflict: Story = {
  args: {
    isOpen: true,
  },
  parameters: {
    docs: {
      description: {
        story: 'Conflict resolver showing a note content conflict with side-by-side diff view.',
      },
    },
  },
};

export const FolderStructureConflict: Story = {
  args: {
    isOpen: true,
  },
  parameters: {
    docs: {
      description: {
        story: 'Conflict resolver showing folder structure conflicts.',
      },
    },
  },
};