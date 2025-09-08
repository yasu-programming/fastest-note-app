'use client';

import React, { useState, useEffect, useRef, useCallback } from 'react';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '@/services/api';
import type { Note, UpdateNoteRequest } from '@/types/note';

interface NoteEditorProps {
  note?: Note;
  folderId?: string;
  onSave?: (note: Note) => void;
  onCancel?: () => void;
  placeholder?: string;
}

export const NoteEditor: React.FC<NoteEditorProps> = ({
  note,
  folderId,
  onSave,
  onCancel,
  placeholder = 'Start typing your note...'
}) => {
  const [title, setTitle] = useState(note?.title || '');
  const [content, setContent] = useState(note?.content || '');
  const [lastSaved, setLastSaved] = useState<Date | null>(note?.updated_at || null);
  const [isSaving, setIsSaving] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  const titleRef = useRef<HTMLInputElement>(null);
  const contentRef = useRef<HTMLTextAreaElement>(null);
  const autoSaveTimeoutRef = useRef<NodeJS.Timeout>();
  const queryClient = useQueryClient();

  const createNoteMutation = useMutation({
    mutationFn: (data: { title: string; content: string; folder_id?: string }) =>
      api.notes.create(data),
    onSuccess: (newNote: Note) => {
      setLastSaved(new Date());
      setHasUnsavedChanges(false);
      queryClient.invalidateQueries({ queryKey: ['notes'] });
      queryClient.invalidateQueries({ queryKey: ['folders'] });
      onSave?.(newNote);
    },
    onError: (error) => {
      console.error('Failed to create note:', error);
    },
  });

  const updateNoteMutation = useMutation({
    mutationFn: (data: UpdateNoteRequest) =>
      api.notes.update(note!.id, data),
    onSuccess: (updatedNote: Note) => {
      setLastSaved(new Date());
      setHasUnsavedChanges(false);
      queryClient.invalidateQueries({ queryKey: ['notes'] });
      queryClient.invalidateQueries({ queryKey: ['note', note!.id] });
      onSave?.(updatedNote);
    },
    onError: (error) => {
      console.error('Failed to update note:', error);
    },
  });

  const autoSave = useCallback(() => {
    if (!title.trim() && !content.trim()) return;
    
    setIsSaving(true);
    
    if (note) {
      updateNoteMutation.mutate({
        title: title || 'Untitled',
        content,
        version: note.version,
      });
    } else if (folderId || title.trim() || content.trim()) {
      createNoteMutation.mutate({
        title: title || 'Untitled',
        content,
        folder_id: folderId,
      });
    }
  }, [title, content, note, folderId, updateNoteMutation, createNoteMutation]);

  const scheduleAutoSave = useCallback(() => {
    if (autoSaveTimeoutRef.current) {
      clearTimeout(autoSaveTimeoutRef.current);
    }
    
    setHasUnsavedChanges(true);
    autoSaveTimeoutRef.current = setTimeout(() => {
      autoSave();
    }, 2000);
  }, [autoSave]);

  const handleTitleChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    setTitle(e.target.value);
    scheduleAutoSave();
  };

  const handleContentChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setContent(e.target.value);
    scheduleAutoSave();
    
    // Auto-resize textarea
    const textarea = e.target;
    textarea.style.height = 'auto';
    textarea.style.height = textarea.scrollHeight + 'px';
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    // Ctrl/Cmd + S for manual save
    if ((e.ctrlKey || e.metaKey) && e.key === 's') {
      e.preventDefault();
      autoSave();
    }
    
    // Tab key in content area for indentation
    if (e.key === 'Tab' && e.target === contentRef.current) {
      e.preventDefault();
      const textarea = contentRef.current!;
      const start = textarea.selectionStart;
      const end = textarea.selectionEnd;
      const newContent = content.substring(0, start) + '    ' + content.substring(end);
      setContent(newContent);
      
      // Restore cursor position
      setTimeout(() => {
        textarea.selectionStart = textarea.selectionEnd = start + 4;
      }, 0);
      
      scheduleAutoSave();
    }
  };

  const handleManualSave = () => {
    if (autoSaveTimeoutRef.current) {
      clearTimeout(autoSaveTimeoutRef.current);
    }
    autoSave();
  };

  useEffect(() => {
    setIsSaving(createNoteMutation.isPending || updateNoteMutation.isPending);
  }, [createNoteMutation.isPending, updateNoteMutation.isPending]);

  useEffect(() => {
    // Focus title input on mount for new notes
    if (!note && titleRef.current) {
      titleRef.current.focus();
    }
  }, [note]);

  useEffect(() => {
    // Cleanup timeout on unmount
    return () => {
      if (autoSaveTimeoutRef.current) {
        clearTimeout(autoSaveTimeoutRef.current);
      }
    };
  }, []);

  const formatLastSaved = (date: Date | null) => {
    if (!date) return 'Never saved';
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    
    if (diff < 60000) return 'Saved just now';
    if (diff < 3600000) return `Saved ${Math.floor(diff / 60000)} minutes ago`;
    if (diff < 86400000) return `Saved ${Math.floor(diff / 3600000)} hours ago`;
    return `Saved on ${date.toLocaleDateString()}`;
  };

  const getStatusColor = () => {
    if (isSaving) return 'text-blue-600';
    if (hasUnsavedChanges) return 'text-orange-600';
    return 'text-green-600';
  };

  const getStatusText = () => {
    if (isSaving) return 'Saving...';
    if (hasUnsavedChanges) return 'Unsaved changes';
    return formatLastSaved(lastSaved);
  };

  return (
    <div className="h-full flex flex-col bg-white">
      {/* Header with save status */}
      <div className="flex items-center justify-between px-6 py-3 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center space-x-4">
          <div className={`text-sm font-medium ${getStatusColor()}`}>
            {isSaving && (
              <svg className="inline w-4 h-4 mr-2 animate-spin" viewBox="0 0 24 24">
                <circle 
                  className="opacity-25" 
                  cx="12" cy="12" r="10" 
                  stroke="currentColor" 
                  strokeWidth="4"
                  fill="none"
                />
                <path 
                  className="opacity-75" 
                  fill="currentColor" 
                  d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"
                />
              </svg>
            )}
            {getStatusText()}
          </div>
        </div>
        
        <div className="flex items-center space-x-2">
          <button
            onClick={handleManualSave}
            disabled={isSaving || (!hasUnsavedChanges && !!note)}
            className="px-3 py-1 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
          >
            Save
          </button>
          {onCancel && (
            <button
              onClick={onCancel}
              className="px-3 py-1 text-sm text-gray-600 hover:text-gray-800 transition-colors"
            >
              Cancel
            </button>
          )}
        </div>
      </div>

      {/* Editor content */}
      <div className="flex-1 flex flex-col p-6">
        {/* Title input */}
        <input
          ref={titleRef}
          type="text"
          value={title}
          onChange={handleTitleChange}
          onKeyDown={handleKeyDown}
          placeholder="Note title..."
          className="w-full text-3xl font-bold text-gray-900 placeholder-gray-400 border-none outline-none bg-transparent resize-none mb-4"
        />

        {/* Content textarea */}
        <textarea
          ref={contentRef}
          value={content}
          onChange={handleContentChange}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          className="flex-1 w-full text-lg text-gray-800 placeholder-gray-400 border-none outline-none bg-transparent resize-none leading-relaxed font-mono"
          style={{ minHeight: '200px' }}
        />
      </div>

      {/* Footer with shortcuts */}
      <div className="px-6 py-2 border-t border-gray-200 bg-gray-50 text-xs text-gray-500">
        <div className="flex items-center justify-between">
          <div className="flex items-center space-x-4">
            <span>Auto-saves every 2 seconds</span>
            <span>•</span>
            <span>Ctrl+S to save manually</span>
            <span>•</span>
            <span>Tab for indentation</span>
          </div>
          <div className="flex items-center space-x-2">
            <span>Characters: {content.length}</span>
            <span>•</span>
            <span>Words: {content.split(/\s+/).filter(Boolean).length}</span>
          </div>
        </div>
      </div>
    </div>
  );
};