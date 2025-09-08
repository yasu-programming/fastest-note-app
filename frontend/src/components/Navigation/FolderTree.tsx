'use client';

import React, { useState, useCallback, useRef } from 'react';
import { useQuery, useMutation, useQueryClient } from '@tanstack/react-query';
import { api } from '@/services/api';
import type { Folder, CreateFolderRequest, UpdateFolderRequest } from '@/types/folder';

interface FolderTreeProps {
  selectedFolderId?: string;
  onFolderSelect?: (folder: Folder | null) => void;
  onFolderCreate?: (folder: Folder) => void;
  onFolderUpdate?: (folder: Folder) => void;
  onFolderDelete?: (folderId: string) => void;
}

interface FolderNodeProps {
  folder: Folder;
  level: number;
  isSelected: boolean;
  isExpanded: boolean;
  onSelect: (folder: Folder) => void;
  onToggleExpand: (folderId: string) => void;
  onRename: (folder: Folder, newName: string) => void;
  onDelete: (folderId: string) => void;
  onCreateChild: (parentId: string, name: string) => void;
  children: Folder[];
}

const FolderNode: React.FC<FolderNodeProps> = ({
  folder,
  level,
  isSelected,
  isExpanded,
  onSelect,
  onToggleExpand,
  onRename,
  onDelete,
  onCreateChild,
  children,
}) => {
  const [isEditing, setIsEditing] = useState(false);
  const [isCreatingChild, setIsCreatingChild] = useState(false);
  const [editName, setEditName] = useState(folder.name);
  const [newChildName, setNewChildName] = useState('');
  const [showContextMenu, setShowContextMenu] = useState(false);
  const [contextMenuPosition, setContextMenuPosition] = useState({ x: 0, y: 0 });
  
  const editInputRef = useRef<HTMLInputElement>(null);
  const newChildInputRef = useRef<HTMLInputElement>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const hasChildren = children.length > 0;
  const indentLevel = level * 20;

  const handleSelect = () => {
    onSelect(folder);
  };

  const handleToggleExpand = (e: React.MouseEvent) => {
    e.stopPropagation();
    if (hasChildren) {
      onToggleExpand(folder.id);
    }
  };

  const handleRename = () => {
    if (editName.trim() && editName !== folder.name) {
      onRename(folder, editName.trim());
    }
    setIsEditing(false);
  };

  const handleCreateChild = () => {
    if (newChildName.trim()) {
      onCreateChild(folder.id, newChildName.trim());
      setNewChildName('');
    }
    setIsCreatingChild(false);
  };

  const handleKeyDown = (e: React.KeyboardEvent, action: 'rename' | 'create') => {
    if (e.key === 'Enter') {
      e.preventDefault();
      if (action === 'rename') {
        handleRename();
      } else {
        handleCreateChild();
      }
    } else if (e.key === 'Escape') {
      if (action === 'rename') {
        setEditName(folder.name);
        setIsEditing(false);
      } else {
        setNewChildName('');
        setIsCreatingChild(false);
      }
    }
  };

  const handleContextMenu = (e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenuPosition({ x: e.clientX, y: e.clientY });
    setShowContextMenu(true);
  };

  const closeContextMenu = () => {
    setShowContextMenu(false);
  };

  const handleContextAction = (action: string) => {
    setShowContextMenu(false);
    
    switch (action) {
      case 'rename':
        setIsEditing(true);
        setTimeout(() => editInputRef.current?.focus(), 0);
        break;
      case 'create':
        setIsCreatingChild(true);
        setTimeout(() => newChildInputRef.current?.focus(), 0);
        break;
      case 'delete':
        if (confirm(`Delete folder "${folder.name}"? This will also delete all notes and subfolders.`)) {
          onDelete(folder.id);
        }
        break;
    }
  };

  // Close context menu when clicking outside
  React.useEffect(() => {
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

  return (
    <>
      <div
        className={`group flex items-center py-1 px-2 hover:bg-gray-100 cursor-pointer relative ${
          isSelected ? 'bg-blue-50 border-r-2 border-blue-500' : ''
        }`}
        style={{ paddingLeft: `${indentLevel + 8}px` }}
        onClick={handleSelect}
        onContextMenu={handleContextMenu}
      >
        {/* Expand/collapse button */}
        <button
          onClick={handleToggleExpand}
          className={`mr-1 w-4 h-4 flex items-center justify-center text-gray-400 hover:text-gray-600 ${
            hasChildren ? 'visible' : 'invisible'
          }`}
        >
          {hasChildren && (
            <svg
              className={`w-3 h-3 transition-transform ${isExpanded ? 'rotate-90' : ''}`}
              fill="currentColor"
              viewBox="0 0 20 20"
            >
              <path d="M7.293 14.707a1 1 0 010-1.414L10.586 10 7.293 6.707a1 1 0 011.414-1.414l4 4a1 1 0 010 1.414l-4 4a1 1 0 01-1.414 0z" />
            </svg>
          )}
        </button>

        {/* Folder icon */}
        <div className="mr-2 text-yellow-600">
          {isExpanded ? (
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
            </svg>
          ) : (
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
            </svg>
          )}
        </div>

        {/* Folder name (editable) */}
        {isEditing ? (
          <input
            ref={editInputRef}
            type="text"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            onBlur={handleRename}
            onKeyDown={(e) => handleKeyDown(e, 'rename')}
            className="flex-1 px-1 py-0 text-sm bg-white border rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
            onClick={(e) => e.stopPropagation()}
          />
        ) : (
          <span className="flex-1 text-sm text-gray-800 truncate">{folder.name}</span>
        )}

        {/* Action buttons (visible on hover) */}
        <div className="opacity-0 group-hover:opacity-100 transition-opacity ml-2">
          <button
            onClick={(e) => {
              e.stopPropagation();
              handleContextAction('create');
            }}
            className="p-1 text-gray-400 hover:text-blue-600 rounded"
            title="Create subfolder"
          >
            <svg className="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
              <path d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" />
            </svg>
          </button>
        </div>
      </div>

      {/* New child folder input */}
      {isCreatingChild && (
        <div
          className="flex items-center py-1 px-2 bg-gray-50"
          style={{ paddingLeft: `${indentLevel + 32}px` }}
        >
          <div className="mr-2 text-yellow-600">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
            </svg>
          </div>
          <input
            ref={newChildInputRef}
            type="text"
            value={newChildName}
            onChange={(e) => setNewChildName(e.target.value)}
            onBlur={handleCreateChild}
            onKeyDown={(e) => handleKeyDown(e, 'create')}
            placeholder="Folder name..."
            className="flex-1 px-1 py-0 text-sm bg-white border rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
          />
        </div>
      )}

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
            onClick={() => handleContextAction('create')}
            className="w-full px-3 py-1 text-left text-sm hover:bg-gray-100"
          >
            New Subfolder
          </button>
          <button
            onClick={() => handleContextAction('rename')}
            className="w-full px-3 py-1 text-left text-sm hover:bg-gray-100"
          >
            Rename
          </button>
          <hr className="my-1" />
          <button
            onClick={() => handleContextAction('delete')}
            className="w-full px-3 py-1 text-left text-sm text-red-600 hover:bg-red-50"
          >
            Delete
          </button>
        </div>
      )}

      {/* Render children */}
      {isExpanded &&
        children.map((child) => (
          <FolderNodeContainer
            key={child.id}
            folder={child}
            level={level + 1}
            isSelected={isSelected}
            onSelect={onSelect}
            onRename={onRename}
            onDelete={onDelete}
            onCreateChild={onCreateChild}
            expandedFolders={[]}
            onToggleExpand={onToggleExpand}
          />
        ))}
    </>
  );
};

interface FolderNodeContainerProps {
  folder: Folder;
  level: number;
  isSelected: boolean;
  expandedFolders: string[];
  onSelect: (folder: Folder) => void;
  onToggleExpand: (folderId: string) => void;
  onRename: (folder: Folder, newName: string) => void;
  onDelete: (folderId: string) => void;
  onCreateChild: (parentId: string, name: string) => void;
}

const FolderNodeContainer: React.FC<FolderNodeContainerProps> = (props) => {
  const { data: folders = [] } = useQuery({
    queryKey: ['folders'],
    queryFn: () => api.folders.list(),
  });

  const children = folders.filter(f => f.parent_id === props.folder.id);

  return (
    <FolderNode
      {...props}
      children={children}
      isExpanded={props.expandedFolders.includes(props.folder.id)}
    />
  );
};

export const FolderTree: React.FC<FolderTreeProps> = ({
  selectedFolderId,
  onFolderSelect,
  onFolderCreate,
  onFolderUpdate,
  onFolderDelete,
}) => {
  const [expandedFolders, setExpandedFolders] = useState<string[]>(['root']);
  const [isCreatingRoot, setIsCreatingRoot] = useState(false);
  const [newRootName, setNewRootName] = useState('');
  const newRootInputRef = useRef<HTMLInputElement>(null);
  
  const queryClient = useQueryClient();

  const { data: folders = [], isLoading, error } = useQuery({
    queryKey: ['folders'],
    queryFn: () => api.folders.list(),
  });

  const createFolderMutation = useMutation({
    mutationFn: (data: CreateFolderRequest) => api.folders.create(data),
    onSuccess: (newFolder: Folder) => {
      queryClient.invalidateQueries({ queryKey: ['folders'] });
      onFolderCreate?.(newFolder);
    },
  });

  const updateFolderMutation = useMutation({
    mutationFn: ({ id, data }: { id: string; data: UpdateFolderRequest }) =>
      api.folders.update(id, data),
    onSuccess: (updatedFolder: Folder) => {
      queryClient.invalidateQueries({ queryKey: ['folders'] });
      onFolderUpdate?.(updatedFolder);
    },
  });

  const deleteFolderMutation = useMutation({
    mutationFn: (id: string) => api.folders.delete(id),
    onSuccess: (_, folderId) => {
      queryClient.invalidateQueries({ queryKey: ['folders'] });
      onFolderDelete?.(folderId);
    },
  });

  const handleToggleExpand = (folderId: string) => {
    setExpandedFolders(prev =>
      prev.includes(folderId)
        ? prev.filter(id => id !== folderId)
        : [...prev, folderId]
    );
  };

  const handleFolderSelect = (folder: Folder | null) => {
    onFolderSelect?.(folder);
  };

  const handleFolderRename = (folder: Folder, newName: string) => {
    updateFolderMutation.mutate({
      id: folder.id,
      data: { name: newName },
    });
  };

  const handleFolderDelete = (folderId: string) => {
    deleteFolderMutation.mutate(folderId);
  };

  const handleCreateChild = (parentId: string, name: string) => {
    createFolderMutation.mutate({
      name,
      parent_id: parentId === 'root' ? null : parentId,
    });
  };

  const handleCreateRoot = () => {
    if (newRootName.trim()) {
      createFolderMutation.mutate({
        name: newRootName.trim(),
        parent_id: null,
      });
      setNewRootName('');
    }
    setIsCreatingRoot(false);
  };

  const handleRootKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') {
      e.preventDefault();
      handleCreateRoot();
    } else if (e.key === 'Escape') {
      setNewRootName('');
      setIsCreatingRoot(false);
    }
  };

  const rootFolders = folders.filter(f => !f.parent_id);

  if (isLoading) {
    return (
      <div className="p-4 text-center">
        <div className="animate-spin w-6 h-6 border-2 border-blue-500 border-t-transparent rounded-full mx-auto mb-2" />
        <p className="text-sm text-gray-500">Loading folders...</p>
      </div>
    );
  }

  if (error) {
    return (
      <div className="p-4 text-center">
        <p className="text-sm text-red-600">Failed to load folders</p>
      </div>
    );
  }

  return (
    <div className="h-full flex flex-col bg-gray-50">
      {/* Header */}
      <div className="flex items-center justify-between p-3 border-b border-gray-200 bg-white">
        <h3 className="font-medium text-gray-900">Folders</h3>
        <button
          onClick={() => {
            setIsCreatingRoot(true);
            setTimeout(() => newRootInputRef.current?.focus(), 0);
          }}
          className="p-1 text-gray-400 hover:text-blue-600 rounded"
          title="Create new folder"
        >
          <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
            <path d="M10 3a1 1 0 011 1v5h5a1 1 0 110 2h-5v5a1 1 0 11-2 0v-5H4a1 1 0 110-2h5V4a1 1 0 011-1z" />
          </svg>
        </button>
      </div>

      {/* Tree content */}
      <div className="flex-1 overflow-y-auto">
        {/* All Notes option */}
        <div
          className={`flex items-center py-2 px-3 hover:bg-gray-100 cursor-pointer ${
            !selectedFolderId ? 'bg-blue-50 border-r-2 border-blue-500' : ''
          }`}
          onClick={() => handleFolderSelect(null)}
        >
          <div className="mr-2 text-gray-600">
            <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
              <path d="M9 2a1 1 0 000 2h2a1 1 0 100-2H9z" />
              <path
                fillRule="evenodd"
                d="M4 5a2 2 0 012-2v1a1 1 0 001 1h6a1 1 0 001-1V3a2 2 0 012 2v6a2 2 0 01-2 2H6a2 2 0 01-2-2V5zm3 4a1 1 0 000 2h.01a1 1 0 100-2H7zm3 0a1 1 0 000 2h3a1 1 0 100-2h-3zm-3 4a1 1 0 100 2h.01a1 1 0 100-2H7zm3 0a1 1 0 100 2h3a1 1 0 100-2h-3z"
                clipRule="evenodd"
              />
            </svg>
          </div>
          <span className="text-sm text-gray-800">All Notes</span>
        </div>

        {/* New root folder input */}
        {isCreatingRoot && (
          <div className="flex items-center py-1 px-3 bg-gray-100">
            <div className="mr-2 text-yellow-600">
              <svg className="w-4 h-4" fill="currentColor" viewBox="0 0 20 20">
                <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
              </svg>
            </div>
            <input
              ref={newRootInputRef}
              type="text"
              value={newRootName}
              onChange={(e) => setNewRootName(e.target.value)}
              onBlur={handleCreateRoot}
              onKeyDown={handleRootKeyDown}
              placeholder="Folder name..."
              className="flex-1 px-1 py-0 text-sm bg-white border rounded focus:outline-none focus:ring-1 focus:ring-blue-500"
            />
          </div>
        )}

        {/* Root folders */}
        {rootFolders.map((folder) => (
          <FolderNodeContainer
            key={folder.id}
            folder={folder}
            level={0}
            isSelected={selectedFolderId === folder.id}
            expandedFolders={expandedFolders}
            onSelect={handleFolderSelect}
            onToggleExpand={handleToggleExpand}
            onRename={handleFolderRename}
            onDelete={handleFolderDelete}
            onCreateChild={handleCreateChild}
          />
        ))}

        {/* Empty state */}
        {rootFolders.length === 0 && !isCreatingRoot && (
          <div className="p-4 text-center text-gray-500">
            <svg className="w-12 h-12 mx-auto mb-2 text-gray-300" fill="currentColor" viewBox="0 0 20 20">
              <path d="M2 6a2 2 0 012-2h5l2 2h5a2 2 0 012 2v6a2 2 0 01-2 2H4a2 2 0 01-2-2V6z" />
            </svg>
            <p className="text-sm">No folders yet</p>
            <button
              onClick={() => {
                setIsCreatingRoot(true);
                setTimeout(() => newRootInputRef.current?.focus(), 0);
              }}
              className="mt-2 text-sm text-blue-600 hover:underline"
            >
              Create your first folder
            </button>
          </div>
        )}
      </div>
    </div>
  );
};