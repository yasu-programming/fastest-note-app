import type { Meta, StoryObj } from '@storybook/react';
import { fn } from '@storybook/test';
import { FolderTree } from './FolderTree';
import type { Folder } from '@/types/folder';

const sampleFolders: Folder[] = [
  {
    id: '1',
    name: 'Personal',
    parent_folder_id: null,
    path: '/personal/',
    level: 0,
    user_id: 'user1',
    item_count: 15,
    created_at: new Date('2023-12-01T10:00:00Z'),
    updated_at: new Date('2023-12-01T10:00:00Z'),
  },
  {
    id: '2', 
    name: 'Work',
    parent_folder_id: null,
    path: '/work/',
    level: 0,
    user_id: 'user1',
    item_count: 25,
    created_at: new Date('2023-12-01T11:00:00Z'),
    updated_at: new Date('2023-12-01T11:00:00Z'),
  },
  {
    id: '3',
    name: 'Projects',
    parent_folder_id: '2',
    path: '/work/projects/',
    level: 1,
    user_id: 'user1',
    item_count: 8,
    created_at: new Date('2023-12-02T09:00:00Z'),
    updated_at: new Date('2023-12-02T09:00:00Z'),
  },
  {
    id: '4',
    name: 'Alpha Project',
    parent_folder_id: '3',
    path: '/work/projects/alpha-project/',
    level: 2,
    user_id: 'user1',
    item_count: 3,
    created_at: new Date('2023-12-03T14:30:00Z'),
    updated_at: new Date('2023-12-03T14:30:00Z'),
  },
];

const meta: Meta<typeof FolderTree> = {
  title: 'Navigation/FolderTree',
  component: FolderTree,
  parameters: {
    layout: 'padded',
    docs: {
      description: {
        component: 'Hierarchical folder tree navigation component supporting up to 10 levels deep. Includes drag-and-drop, inline editing, and context menus.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    selectedFolderId: {
      description: 'ID of currently selected folder',
      control: { type: 'text' },
    },
    onFolderSelect: {
      description: 'Callback when a folder is selected',
    },
    onFolderCreate: {
      description: 'Callback when a new folder is created',
    },
    onFolderUpdate: {
      description: 'Callback when a folder is updated',
    },
    onFolderDelete: {
      description: 'Callback when a folder is deleted',
    },
  },
  args: {
    onFolderSelect: fn(),
    onFolderCreate: fn(),
    onFolderUpdate: fn(),
    onFolderDelete: fn(),
  },
};

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Default folder tree with sample folder hierarchy.',
      },
    },
  },
};

export const WithSelectedFolder: Story = {
  args: {
    selectedFolderId: '3',
  },
  parameters: {
    docs: {
      description: {
        story: 'Folder tree with a selected folder highlighted.',
      },
    },
  },
};

export const DeepHierarchy: Story = {
  args: {
    selectedFolderId: '4',
  },
  parameters: {
    docs: {
      description: {
        story: 'Folder tree showing deep hierarchy with nested folders.',
      },
    },
  },
};

export const Empty: Story = {
  args: {},
  parameters: {
    docs: {
      description: {
        story: 'Empty folder tree when user has no folders yet.',
      },
    },
  },
};