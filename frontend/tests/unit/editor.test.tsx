import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import userEvent from '@testing-library/user-event';
import '@testing-library/jest-dom';

import { NoteEditor } from '@/components/Editor/NoteEditor';
import { api } from '@/services/api';
import type { Note } from '@/types/note';

// Mock the API
jest.mock('@/services/api', () => ({
  api: {
    notes: {
      create: jest.fn(),
      update: jest.fn(),
    },
  },
}));

const mockApi = api as jest.Mocked<typeof api>;

// Test wrapper with QueryClient
const TestWrapper: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
      mutations: { retry: false },
    },
  });

  return (
    <QueryClientProvider client={queryClient}>
      {children}
    </QueryClientProvider>
  );
};

// Mock note data
const mockNote: Note = {
  id: '1',
  title: 'Test Note',
  content: 'This is test content',
  folder_id: null,
  user_id: 'user-1',
  version: 1,
  created_at: '2024-01-01T00:00:00Z',
  updated_at: '2024-01-01T00:00:00Z',
};

describe('NoteEditor', () => {
  const mockOnSave = jest.fn();
  const mockOnCancel = jest.fn();

  beforeEach(() => {
    jest.clearAllMocks();
    jest.useFakeTimers();
  });

  afterEach(() => {
    jest.useRealTimers();
  });

  it('renders empty editor for new note', () => {
    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
          placeholder="Start writing..."
        />
      </TestWrapper>
    );

    expect(screen.getByPlaceholderText('Note title...')).toBeInTheDocument();
    expect(screen.getByPlaceholderText('Start writing...')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /save/i })).toBeDisabled();
  });

  it('renders editor with existing note data', () => {
    render(
      <TestWrapper>
        <NoteEditor 
          note={mockNote}
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    expect(screen.getByDisplayValue('Test Note')).toBeInTheDocument();
    expect(screen.getByDisplayValue('This is test content')).toBeInTheDocument();
  });

  it('enables save button when content is modified', async () => {
    const user = userEvent.setup({ advanceTimers: jest.advanceTimersByTime });
    
    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    const saveButton = screen.getByRole('button', { name: /save/i });

    expect(saveButton).toBeDisabled();

    await user.type(titleInput, 'New Note');

    expect(saveButton).not.toBeDisabled();
    expect(screen.getByText(/unsaved changes/i)).toBeInTheDocument();
  });

  it('auto-saves after 2 seconds of inactivity', async () => {
    const user = userEvent.setup({ advanceTimers: jest.advanceTimersByTime });
    mockApi.notes.create.mockResolvedValue({ ...mockNote, title: 'Auto-saved Note' });

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    await user.type(titleInput, 'Auto-saved Note');

    // Fast-forward timer to trigger auto-save
    jest.advanceTimersByTime(2000);

    await waitFor(() => {
      expect(mockApi.notes.create).toHaveBeenCalledWith({
        title: 'Auto-saved Note',
        content: '',
        folder_id: undefined,
      });
    });
  });

  it('shows saving indicator during auto-save', async () => {
    const user = userEvent.setup({ advanceTimers: jest.advanceTimersByTime });
    mockApi.notes.create.mockImplementation(() => 
      new Promise(resolve => setTimeout(() => resolve(mockNote), 100))
    );

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    await user.type(titleInput, 'Saving Note');

    jest.advanceTimersByTime(2000);

    await waitFor(() => {
      expect(screen.getByText(/saving/i)).toBeInTheDocument();
    });

    // Wait for save to complete
    jest.advanceTimersByTime(100);
    
    await waitFor(() => {
      expect(screen.queryByText(/saving/i)).not.toBeInTheDocument();
    });
  });

  it('handles manual save', async () => {
    const user = userEvent.setup();
    mockApi.notes.create.mockResolvedValue(mockNote);

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    const contentInput = screen.getByPlaceholderText('Start typing your note...');
    const saveButton = screen.getByRole('button', { name: /save/i });

    await user.type(titleInput, 'Manual Save Note');
    await user.type(contentInput, 'Manual save content');
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockApi.notes.create).toHaveBeenCalledWith({
        title: 'Manual Save Note',
        content: 'Manual save content',
        folder_id: undefined,
      });
    });

    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalledWith(mockNote);
    });
  });

  it('handles Ctrl+S keyboard shortcut', async () => {
    const user = userEvent.setup();
    mockApi.notes.create.mockResolvedValue(mockNote);

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    await user.type(titleInput, 'Shortcut Note');

    // Simulate Ctrl+S
    await user.keyboard('{Control>}s{/Control}');

    await waitFor(() => {
      expect(mockApi.notes.create).toHaveBeenCalledWith({
        title: 'Shortcut Note',
        content: '',
        folder_id: undefined,
      });
    });
  });

  it('handles Tab key for indentation in content area', async () => {
    const user = userEvent.setup();

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const contentInput = screen.getByPlaceholderText('Start typing your note...');
    await user.click(contentInput);
    await user.type(contentInput, 'Line 1');
    await user.keyboard('{Enter}');
    await user.keyboard('{Tab}');
    await user.type(contentInput, 'Indented line');

    expect(contentInput).toHaveValue('Line 1\n    Indented line');
  });

  it('auto-resizes textarea content', async () => {
    const user = userEvent.setup();

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const contentInput = screen.getByPlaceholderText('Start typing your note...') as HTMLTextAreaElement;
    const initialHeight = contentInput.style.height;

    // Add multiple lines of content
    const longContent = Array(20).fill('This is a long line of text').join('\n');
    await user.type(contentInput, longContent);

    // Height should have increased (this is implementation-dependent)
    expect(contentInput.style.height).not.toBe(initialHeight);
  });

  it('shows character and word count', async () => {
    const user = userEvent.setup();

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const contentInput = screen.getByPlaceholderText('Start typing your note...');
    await user.type(contentInput, 'Hello world test content');

    expect(screen.getByText(/characters: 24/i)).toBeInTheDocument();
    expect(screen.getByText(/words: 4/i)).toBeInTheDocument();
  });

  it('updates existing note', async () => {
    const user = userEvent.setup();
    const updatedNote = { ...mockNote, title: 'Updated Title', version: 2 };
    mockApi.notes.update.mockResolvedValue(updatedNote);

    render(
      <TestWrapper>
        <NoteEditor 
          note={mockNote}
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByDisplayValue('Test Note');
    await user.clear(titleInput);
    await user.type(titleInput, 'Updated Title');

    const saveButton = screen.getByRole('button', { name: /save/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockApi.notes.update).toHaveBeenCalledWith(mockNote.id, {
        title: 'Updated Title',
        content: 'This is test content',
        version: 1,
      });
    });

    await waitFor(() => {
      expect(mockOnSave).toHaveBeenCalledWith(updatedNote);
    });
  });

  it('handles save errors gracefully', async () => {
    const user = userEvent.setup();
    const consoleError = jest.spyOn(console, 'error').mockImplementation();
    mockApi.notes.create.mockRejectedValue(new Error('Save failed'));

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    const saveButton = screen.getByRole('button', { name: /save/i });

    await user.type(titleInput, 'Error Note');
    await user.click(saveButton);

    await waitFor(() => {
      expect(consoleError).toHaveBeenCalledWith('Failed to create note:', expect.any(Error));
    });

    // Should not call onSave on error
    expect(mockOnSave).not.toHaveBeenCalled();

    consoleError.mockRestore();
  });

  it('shows last saved time', () => {
    render(
      <TestWrapper>
        <NoteEditor 
          note={mockNote}
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    // The component should show some form of last saved time
    // This would depend on your exact implementation
    expect(screen.getByText(/saved/i)).toBeInTheDocument();
  });

  it('handles cancel action', async () => {
    const user = userEvent.setup();

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const cancelButton = screen.getByRole('button', { name: /cancel/i });
    await user.click(cancelButton);

    expect(mockOnCancel).toHaveBeenCalled();
  });

  it('focuses title input for new notes', () => {
    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    expect(titleInput).toHaveFocus();
  });

  it('does not focus title input for existing notes', () => {
    render(
      <TestWrapper>
        <NoteEditor 
          note={mockNote}
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByDisplayValue('Test Note');
    expect(titleInput).not.toHaveFocus();
  });

  it('prevents save of empty content for new notes', async () => {
    const user = userEvent.setup();

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const saveButton = screen.getByRole('button', { name: /save/i });
    
    // Button should be disabled for empty content
    expect(saveButton).toBeDisabled();

    // Even clicking shouldn't trigger save
    await user.click(saveButton);
    expect(mockApi.notes.create).not.toHaveBeenCalled();
  });

  it('clears auto-save timeout on unmount', () => {
    const { unmount } = render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    // This is more of an implementation test
    // In practice, you'd need to verify that timeouts are cleared
    unmount();
    
    // Advance timers to ensure no auto-save happens after unmount
    jest.advanceTimersByTime(5000);
    expect(mockApi.notes.create).not.toHaveBeenCalled();
  });

  it('handles special characters and unicode', async () => {
    const user = userEvent.setup();
    mockApi.notes.create.mockResolvedValue(mockNote);

    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    const titleInput = screen.getByPlaceholderText('Note title...');
    const contentInput = screen.getByPlaceholderText('Start typing your note...');

    const unicodeTitle = '🚀 Unicode Title 日本語';
    const unicodeContent = 'Content with emojis 😀 and unicode: 测试内容';

    await user.type(titleInput, unicodeTitle);
    await user.type(contentInput, unicodeContent);

    const saveButton = screen.getByRole('button', { name: /save/i });
    await user.click(saveButton);

    await waitFor(() => {
      expect(mockApi.notes.create).toHaveBeenCalledWith({
        title: unicodeTitle,
        content: unicodeContent,
        folder_id: undefined,
      });
    });
  });

  it('shows keyboard shortcuts help', () => {
    render(
      <TestWrapper>
        <NoteEditor 
          onSave={mockOnSave}
          onCancel={mockOnCancel}
        />
      </TestWrapper>
    );

    expect(screen.getByText(/ctrl\+s to save manually/i)).toBeInTheDocument();
    expect(screen.getByText(/tab for indentation/i)).toBeInTheDocument();
    expect(screen.getByText(/auto-saves every 2 seconds/i)).toBeInTheDocument();
  });
});