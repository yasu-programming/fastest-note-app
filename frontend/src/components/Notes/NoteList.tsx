'use client';

import React, { useState, useEffect, useMemo, useRef, useCallback } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '@/services/api';
import type { Note } from '@/types/note';

interface NoteListProps {
  folderId?: string;
  selectedNoteId?: string;
  onNoteSelect?: (note: Note) => void;
  onNoteCreate?: (note: Note) => void;
  onNoteDelete?: (noteId: string) => void;
  searchQuery?: string;
  sortBy?: 'updated_at' | 'created_at' | 'title';
  sortOrder?: 'asc' | 'desc';
}

interface VirtualizedListProps {
  items: Note[];
  itemHeight: number;
  containerHeight: number;
  renderItem: (item: Note, index: number) => React.ReactNode;
}

const VirtualizedList: React.FC<VirtualizedListProps> = ({
  items,
  itemHeight,
  containerHeight,
  renderItem,
}) => {
  const [scrollTop, setScrollTop] = useState(0);
  const containerRef = useRef<HTMLDivElement>(null);

  const visibleRange = useMemo(() => {
    const start = Math.floor(scrollTop / itemHeight);
    const end = Math.min(
      start + Math.ceil(containerHeight / itemHeight) + 1,
      items.length
    );
    return { start: Math.max(0, start), end };
  }, [scrollTop, itemHeight, containerHeight, items.length]);

  const handleScroll = useCallback((e: React.UIEvent<HTMLDivElement>) => {
    setScrollTop(e.currentTarget.scrollTop);
  }, []);

  const totalHeight = items.length * itemHeight;
  const offsetY = visibleRange.start * itemHeight;

  return (
    <div
      ref={containerRef}
      className="overflow-auto"
      style={{ height: containerHeight }}
      onScroll={handleScroll}
    >
      <div style={{ height: totalHeight, position: 'relative' }}>
        <div style={{ transform: `translateY(${offsetY}px)` }}>
          {items.slice(visibleRange.start, visibleRange.end).map((item, index) =>
            renderItem(item, visibleRange.start + index)
          )}
        </div>
      </div>
    </div>
  );
};

interface NoteItemProps {
  note: Note;
  isSelected: boolean;
  onSelect: (note: Note) => void;
  onDelete: (noteId: string) => void;
  searchQuery?: string;
}

const NoteItem: React.FC<NoteItemProps> = ({
  note,
  isSelected,
  onSelect,
  onDelete,
  searchQuery,
}) => {
  const [showContextMenu, setShowContextMenu] = useState(false);
  const [contextMenuPosition, setContextMenuPosition] = useState({ x: 0, y: 0 });
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenuPosition({ x: e.clientX, y: e.clientY });
    setShowContextMenu(true);
  };

  const closeContextMenu = () => {
    setShowContextMenu(false);
  };

  const handleDelete = () => {
    setShowContextMenu(false);
    if (confirm(`Delete note "${note.title}"?`)) {
      onDelete(note.id);
    }
  };

  // Close context menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (contextMenuRef.current && !contextMenuRef.current.contains(e.target as Node)) {
        closeContextMenu();
      }
    };

    if (showContextMenu) {
      document.addEventListener('mousedown', handleClickOutside);
      return () => document.removeEventListener('mousedown', handleClickOutside);
    }
  }, [showContextMenu]);

  const highlightText = (text: string, query?: string) => {
    if (!query) return text;
    const regex = new RegExp(`(${query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
    return text.replace(regex, '<mark>$1</mark>');
  };

  const getPreviewText = (content: string, maxLength = 100) => {
    const text = content.replace(/\n/g, ' ').trim();
    return text.length > maxLength ? text.substring(0, maxLength) + '...' : text;
  };

  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    
    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
    if (diff < 604800000) return `${Math.floor(diff / 86400000)}d ago`;
    
    return date.toLocaleDateString();
  };

  return (
    <>
      <div
        className={`group p-4 border-b border-gray-200 cursor-pointer hover:bg-gray-50 transition-colors ${
          isSelected ? 'bg-blue-50 border-l-4 border-l-blue-500' : ''
        }`}
        onClick={() => onSelect(note)}
        onContextMenu={handleContextMenu}
        style={{ height: '120px' }}
      >
        <div className="flex flex-col h-full">
          {/* Title and date */}
          <div className="flex items-start justify-between mb-2">
            <h3
              className="font-medium text-gray-900 truncate flex-1 mr-2"
              dangerouslySetInnerHTML={{
                __html: highlightText(note.title || 'Untitled', searchQuery),
              }}
            />
            <span className="text-xs text-gray-500 whitespace-nowrap">
              {formatDate(note.updated_at)}
            </span>
          </div>

          {/* Content preview */}
          <div className="flex-1 mb-2">
            <p
              className="text-sm text-gray-600 line-clamp-3 leading-relaxed"
              dangerouslySetInnerHTML={{
                __html: highlightText(getPreviewText(note.content), searchQuery),
              }}
            />
          </div>

          {/* Footer with metadata */}
          <div className="flex items-center justify-between text-xs text-gray-400">
            <div className="flex items-center space-x-3">
              {note.folder_id && (
                <span className="flex items-center">
                  <svg className="w-3 h-3 mr-1" fill="currentColor" viewBox="0 0 20 20">
                    <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
                  </svg>
                  Folder
                </span>
              )}
              <span>{note.content.length} chars</span>
              <span>{note.content.split(/\s+/).filter(Boolean).length} words</span>
            </div>
            
            <div className="opacity-0 group-hover:opacity-100 transition-opacity">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleDelete();
                }}
                className="p-1 text-red-400 hover:text-red-600 rounded"
                title="Delete note"
              >
                <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                  <path
                    fillRule="evenodd"
                    d="M9 2a1 1 0 00-.894.553L7.382 4H4a1 1 0 000 2v10a2 2 0 002 2h8a2 2 0 002-2V6a1 1 0 100-2h-3.382l-.724-1.447A1 1 0 0011 2H9zM7 8a1 1 0 012 0v6a1 1 0 11-2 0V8zm5-1a1 1 0 00-1 1v6a1 1 0 102 0V8a1 1 0 00-1-1z"
                    clipRule="evenodd"
                  />
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Context menu */}
      {showContextMenu && (
        <div
          ref={contextMenuRef}
          className="fixed z-50 bg-white border rounded-md shadow-lg py-1 min-w-32"
          style={{
            left: `${contextMenuPosition.x}px`,
            top: `${contextMenuPosition.y}px`,
          }}
        >
          <button
            onClick={() => {
              closeContextMenu();
              onSelect(note);
            }}
            className="w-full px-3 py-1 text-left text-sm hover:bg-gray-100"
          >
            Open
          </button>
          <button
            onClick={() => {
              closeContextMenu();
              navigator.clipboard.writeText(note.content);
            }}
            className="w-full px-3 py-1 text-left text-sm hover:bg-gray-100"
          >
            Copy Content
          </button>
          <hr className="my-1" />
          <button
            onClick={handleDelete}
            className="w-full px-3 py-1 text-left text-sm text-red-600 hover:bg-red-50"
          >
            Delete
          </button>
        </div>
      )}
    </>
  );
};

export const NoteList: React.FC<NoteListProps> = ({
  folderId,
  selectedNoteId,
  onNoteSelect,
  onNoteCreate,
  onNoteDelete,
  searchQuery = '',
  sortBy = 'updated_at',
  sortOrder = 'desc',
}) => {
  const [containerHeight, setContainerHeight] = useState(600);
  const containerRef = useRef<HTMLDivElement>(null);
  const queryClient = useQueryClient();

  const { data: notes = [], isLoading, error } = useQuery({
    queryKey: ['notes', folderId, searchQuery],
    queryFn: () => api.notes.list({ folder_id: folderId, search: searchQuery }),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: string) => api.notes.delete(id),
    onSuccess: (_, noteId) => {
      queryClient.invalidateQueries({ queryKey: ['notes'] });
      onNoteDelete?.(noteId);
    },
  });

  // Filter and sort notes
  const filteredAndSortedNotes = useMemo(() => {
    let filtered = notes;

    // Filter by search query
    if (searchQuery.trim()) {
      const query = searchQuery.toLowerCase();
      filtered = notes.filter(
        (note) =>
          note.title.toLowerCase().includes(query) ||
          note.content.toLowerCase().includes(query)
      );
    }

    // Sort notes
    filtered.sort((a, b) => {
      let aValue: string | Date;
      let bValue: string | Date;

      switch (sortBy) {
        case 'title':
          aValue = a.title.toLowerCase();
          bValue = b.title.toLowerCase();
          break;
        case 'created_at':
          aValue = new Date(a.created_at);
          bValue = new Date(b.created_at);
          break;
        default:
          aValue = new Date(a.updated_at);
          bValue = new Date(b.updated_at);
      }

      if (sortOrder === 'asc') {
        return aValue < bValue ? -1 : aValue > bValue ? 1 : 0;
      } else {
        return aValue > bValue ? -1 : aValue < bValue ? 1 : 0;
      }
    });

    return filtered;
  }, [notes, searchQuery, sortBy, sortOrder]);

  // Handle container resize
  useEffect(() => {
    const handleResize = () => {
      if (containerRef.current) {
        setContainerHeight(containerRef.current.clientHeight);
      }
    };

    handleResize();
    window.addEventListener('resize', handleResize);
    return () => window.removeEventListener('resize', handleResize);
  }, []);

  const handleNoteSelect = (note: Note) => {
    onNoteSelect?.(note);
  };

  const handleNoteDelete = (noteId: string) => {
    deleteMutation.mutate(noteId);
  };

  const renderNoteItem = (note: Note, index: number) => (
    <NoteItem
      key={`${note.id}-${index}`}
      note={note}
      isSelected={selectedNoteId === note.id}
      onSelect={handleNoteSelect}
      onDelete={handleNoteDelete}
      searchQuery={searchQuery}
    />
  );

  if (isLoading) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="animate-spin w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-3" />
          <p className="text-sm text-gray-500">Loading notes...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <svg className="w-12 h-12 mx-auto mb-3 text-red-400" fill="currentColor" viewBox="0 0 20 20">
            <path
              fillRule="evenodd"
              d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
              clipRule="evenodd"
            />
          </svg>
          <p className="text-sm text-red-600">Failed to load notes</p>
          <button
            onClick={() => queryClient.invalidateQueries({ queryKey: ['notes'] })}
            className="mt-2 text-sm text-blue-600 hover:underline"
          >
            Try again
          </button>
        </div>
      </div>
    );
  }

  return (
    <div ref={containerRef} className="h-full flex flex-col bg-white">
      {/* Header with stats */}
      <div className="flex items-center justify-between p-4 border-b border-gray-200 bg-gray-50">
        <div className="flex items-center space-x-4">
          <h2 className="font-medium text-gray-900">
            {folderId ? 'Folder Notes' : 'All Notes'}
          </h2>
          <span className="text-sm text-gray-500">
            {filteredAndSortedNotes.length} note{filteredAndSortedNotes.length !== 1 ? 's' : ''}
            {searchQuery && ` matching "${searchQuery}"`}
          </span>
        </div>
        
        {deleteMutation.isPending && (
          <div className="flex items-center text-sm text-gray-500">
            <svg className="animate-spin w-4 h-4 mr-2" viewBox="0 0 24 24">
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
            Deleting...
          </div>
        )}
      </div>

      {/* Notes list */}
      {filteredAndSortedNotes.length === 0 ? (
        <div className="flex-1 flex items-center justify-center">
          <div className="text-center">
            <svg className="w-16 h-16 mx-auto mb-4 text-gray-300" fill="currentColor" viewBox="0 0 20 20">
              <path d="M9 2a1 1 0 000 2h2a1 1 0 100-2H9z" />
              <path
                fillRule="evenodd"
                d="M4 5a2 2 0 012-2v1a1 1 0 001 1h6a1 1 0 001-1V3a2 2 0 012 2v6a2 2 0 01-2 2H6a2 2 0 01-2-2V5zm3 4a1 1 0 000 2h.01a1 1 0 100-2H7zm3 0a1 1 0 000 2h3a1 1 0 100-2h-3zm-3 4a1 1 0 100 2h.01a1 1 0 100-2H7zm3 0a1 1 0 100 2h3a1 1 0 100-2h-3z"
                clipRule="evenodd"
              />
            </svg>
            <h3 className="text-lg font-medium text-gray-900 mb-2">
              {searchQuery ? 'No matching notes' : 'No notes yet'}
            </h3>
            <p className="text-gray-500 mb-4">
              {searchQuery 
                ? `No notes found matching "${searchQuery}"`
                : folderId 
                  ? 'This folder is empty. Create your first note!'
                  : 'Start by creating your first note.'
              }
            </p>
          </div>
        </div>
      ) : (
        <div className="flex-1">
          <VirtualizedList
            items={filteredAndSortedNotes}
            itemHeight={120}
            containerHeight={containerHeight - 80} // Subtract header height
            renderItem={renderNoteItem}
          />
        </div>
      )}
    </div>
  );
};