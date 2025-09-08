'use client';

import React, { useState, useEffect } from 'react';
import { useSyncQueue } from '@/services/syncQueue';
import { useContent } from '@/stores/contentStore';
import type { OfflineOperation } from '@/services/offline';
import type { Note } from '@/types/note';
import type { Folder } from '@/types/folder';

interface ConflictData {
  operationId: string;
  localData: any;
  remoteData: any;
  field: string;
}

interface ConflictItem {
  operation: OfflineOperation;
  conflicts: ConflictData[];
}

interface ConflictResolverProps {
  isOpen: boolean;
  onClose: () => void;
  onAllResolved?: () => void;
}

interface ConflictComparisonProps {
  title: string;
  localValue: any;
  remoteValue: any;
  onChoose: (choice: 'local' | 'remote' | 'merge') => void;
  chosen?: 'local' | 'remote' | 'merge';
  mergeValue?: any;
  onMergeChange?: (value: any) => void;
  fieldType?: 'text' | 'textarea' | 'select';
}

const ConflictComparison: React.FC<ConflictComparisonProps> = ({
  title,
  localValue,
  remoteValue,
  onChoose,
  chosen,
  mergeValue,
  onMergeChange,
  fieldType = 'text',
}) => {
  const formatValue = (value: any) => {
    if (typeof value === 'string') return value;
    if (typeof value === 'object') return JSON.stringify(value, null, 2);
    return String(value);
  };

  const renderValue = (value: any, type: 'local' | 'remote' | 'merge') => {
    const formattedValue = formatValue(value);
    
    if (fieldType === 'textarea' || formattedValue.includes('\n')) {
      return (
        <textarea
          value={type === 'merge' ? (mergeValue ?? formattedValue) : formattedValue}
          onChange={(e) => type === 'merge' && onMergeChange?.(e.target.value)}
          readOnly={type !== 'merge'}
          className={`w-full h-32 p-3 border rounded-md font-mono text-sm resize-none ${
            type === 'merge' 
              ? 'border-purple-300 focus:border-purple-500 focus:ring-1 focus:ring-purple-500'
              : 'bg-gray-50 border-gray-300'
          }`}
          placeholder={type === 'merge' ? 'Edit merged content...' : ''}
        />
      );
    }

    return (
      <input
        type="text"
        value={type === 'merge' ? (mergeValue ?? formattedValue) : formattedValue}
        onChange={(e) => type === 'merge' && onMergeChange?.(e.target.value)}
        readOnly={type !== 'merge'}
        className={`w-full p-3 border rounded-md font-mono text-sm ${
          type === 'merge' 
            ? 'border-purple-300 focus:border-purple-500 focus:ring-1 focus:ring-purple-500'
            : 'bg-gray-50 border-gray-300'
        }`}
        placeholder={type === 'merge' ? 'Edit merged content...' : ''}
      />
    );
  };

  return (
    <div className="border rounded-lg p-4 bg-white">
      <h4 className="font-medium text-gray-900 mb-3">{title}</h4>
      
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Local Version */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-blue-700">Your Version (Local)</label>
            <button
              onClick={() => onChoose('local')}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                chosen === 'local'
                  ? 'bg-blue-600 text-white'
                  : 'bg-blue-100 text-blue-700 hover:bg-blue-200'
              }`}
            >
              Use This
            </button>
          </div>
          {renderValue(localValue, 'local')}
        </div>

        {/* Remote Version */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-green-700">Server Version (Remote)</label>
            <button
              onClick={() => onChoose('remote')}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                chosen === 'remote'
                  ? 'bg-green-600 text-white'
                  : 'bg-green-100 text-green-700 hover:bg-green-200'
              }`}
            >
              Use This
            </button>
          </div>
          {renderValue(remoteValue, 'remote')}
        </div>

        {/* Merge Option */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="text-sm font-medium text-purple-700">Merged Version</label>
            <button
              onClick={() => onChoose('merge')}
              className={`px-3 py-1 text-xs rounded transition-colors ${
                chosen === 'merge'
                  ? 'bg-purple-600 text-white'
                  : 'bg-purple-100 text-purple-700 hover:bg-purple-200'
              }`}
            >
              Use This
            </button>
          </div>
          {renderValue(localValue, 'merge')}
          <p className="text-xs text-gray-500">
            Edit the content above to create a merged version
          </p>
        </div>
      </div>
    </div>
  );
};

interface ConflictItemComponentProps {
  conflict: ConflictItem;
  onResolve: (operationId: string, resolution: 'accept' | 'reject', mergedData?: any) => void;
}

const ConflictItemComponent: React.FC<ConflictItemComponentProps> = ({
  conflict,
  onResolve,
}) => {
  const [resolutions, setResolutions] = useState<Record<string, {
    choice: 'local' | 'remote' | 'merge';
    mergeValue?: any;
  }>>({});

  const { operation } = conflict;
  const entityName = operation.entityType === 'note' ? 'Note' : 'Folder';
  const operationName = operation.type.charAt(0).toUpperCase() + operation.type.slice(1);

  const handleChoiceChange = (conflictId: string, choice: 'local' | 'remote' | 'merge') => {
    setResolutions(prev => ({
      ...prev,
      [conflictId]: {
        ...prev[conflictId],
        choice,
        mergeValue: prev[conflictId]?.mergeValue,
      },
    }));
  };

  const handleMergeChange = (conflictId: string, mergeValue: any) => {
    setResolutions(prev => ({
      ...prev,
      [conflictId]: {
        ...prev[conflictId],
        mergeValue,
      },
    }));
  };

  const handleResolve = (accept: boolean) => {
    if (accept) {
      const mergedData = { ...operation.data };
      
      for (const conflictData of conflict.conflicts) {
        const resolution = resolutions[conflictData.operationId];
        if (resolution) {
          if (resolution.choice === 'local') {
            mergedData[conflictData.field] = conflictData.localData[conflictData.field];
          } else if (resolution.choice === 'remote') {
            mergedData[conflictData.field] = conflictData.remoteData[conflictData.field];
          } else if (resolution.choice === 'merge') {
            mergedData[conflictData.field] = resolution.mergeValue;
          }
        }
      }
      
      onResolve(operation.id, 'accept', mergedData);
    } else {
      onResolve(operation.id, 'reject');
    }
  };

  const allConflictsResolved = conflict.conflicts.every(c => 
    resolutions[c.operationId]?.choice !== undefined
  );

  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp).toLocaleString();
  };

  return (
    <div className="border border-orange-200 rounded-lg p-6 bg-orange-50">
      <div className="flex items-start justify-between mb-4">
        <div>
          <h3 className="text-lg font-semibold text-gray-900">
            {operationName} {entityName} Conflict
          </h3>
          <p className="text-sm text-gray-600 mt-1">
            Operation from {formatTimestamp(operation.timestamp)}
          </p>
          {operation.lastError && (
            <p className="text-sm text-red-600 mt-1">
              Error: {operation.lastError}
            </p>
          )}
        </div>
        <div className="flex items-center space-x-2">
          <span className="px-2 py-1 text-xs bg-orange-200 text-orange-800 rounded">
            {operation.retryCount} retries
          </span>
          <span className="px-2 py-1 text-xs bg-red-200 text-red-800 rounded">
            Conflict
          </span>
        </div>
      </div>

      <div className="space-y-4 mb-6">
        {conflict.conflicts.map((conflictData) => (
          <ConflictComparison
            key={conflictData.operationId}
            title={`${conflictData.field.charAt(0).toUpperCase() + conflictData.field.slice(1)} Conflict`}
            localValue={conflictData.localData}
            remoteValue={conflictData.remoteData}
            onChoose={(choice) => handleChoiceChange(conflictData.operationId, choice)}
            chosen={resolutions[conflictData.operationId]?.choice}
            mergeValue={resolutions[conflictData.operationId]?.mergeValue}
            onMergeChange={(value) => handleMergeChange(conflictData.operationId, value)}
            fieldType={conflictData.field === 'content' ? 'textarea' : 'text'}
          />
        ))}
      </div>

      <div className="flex items-center justify-between pt-4 border-t border-orange-200">
        <div className="text-sm text-gray-600">
          {allConflictsResolved ? (
            <span className="text-green-600 font-medium">
              ✓ All conflicts resolved
            </span>
          ) : (
            <span>
              Please resolve all conflicts before continuing
            </span>
          )}
        </div>
        
        <div className="flex items-center space-x-3">
          <button
            onClick={() => handleResolve(false)}
            className="px-4 py-2 text-sm text-gray-700 bg-gray-200 hover:bg-gray-300 rounded-md transition-colors"
          >
            Discard Changes
          </button>
          <button
            onClick={() => handleResolve(true)}
            disabled={!allConflictsResolved}
            className="px-4 py-2 text-sm bg-blue-600 text-white rounded-md hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors"
          >
            Apply Resolution
          </button>
        </div>
      </div>
    </div>
  );
};

export const ConflictResolver: React.FC<ConflictResolverProps> = ({
  isOpen,
  onClose,
  onAllResolved,
}) => {
  const [conflicts, setConflicts] = useState<ConflictItem[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const { resolveConflict } = useContent();
  const { retryOperation, cancelOperation, getSyncStats } = useSyncQueue();

  useEffect(() => {
    if (isOpen) {
      loadConflicts();
    }
  }, [isOpen]);

  const loadConflicts = async () => {
    setIsLoading(true);
    try {
      // This would typically come from the sync queue service
      // For now, we'll simulate conflict data
      const mockConflicts: ConflictItem[] = [];
      setConflicts(mockConflicts);
    } catch (error) {
      console.error('Failed to load conflicts:', error);
    } finally {
      setIsLoading(false);
    }
  };

  const handleResolve = async (operationId: string, resolution: 'accept' | 'reject', mergedData?: any) => {
    try {
      if (resolution === 'accept') {
        if (mergedData) {
          // Update the operation with merged data before retrying
          // This would require extending the sync queue service
        }
        await retryOperation(operationId);
      } else {
        await cancelOperation(operationId);
      }

      setConflicts(prev => prev.filter(c => c.operation.id !== operationId));

      if (conflicts.length <= 1) {
        onAllResolved?.();
        onClose();
      }
    } catch (error) {
      console.error('Failed to resolve conflict:', error);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 overflow-y-auto">
      <div className="flex items-center justify-center min-h-screen px-4 pt-4 pb-20 text-center sm:block sm:p-0">
        <div
          className="fixed inset-0 bg-gray-500 bg-opacity-75 transition-opacity"
          onClick={onClose}
        />

        <div className="relative inline-block w-full max-w-6xl p-6 my-8 text-left align-middle bg-white rounded-lg shadow-xl transform transition-all sm:align-middle">
          <div className="flex items-center justify-between mb-6">
            <div>
              <h2 className="text-2xl font-bold text-gray-900">
                Resolve Sync Conflicts
              </h2>
              <p className="text-sm text-gray-600 mt-1">
                {conflicts.length} conflict{conflicts.length !== 1 ? 's' : ''} need{conflicts.length === 1 ? 's' : ''} your attention
              </p>
            </div>
            <button
              onClick={onClose}
              className="p-2 text-gray-400 hover:text-gray-600 rounded-md"
            >
              <svg className="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <div className="animate-spin w-8 h-8 border-2 border-blue-500 border-t-transparent rounded-full" />
              <span className="ml-3 text-gray-600">Loading conflicts...</span>
            </div>
          ) : conflicts.length === 0 ? (
            <div className="text-center py-12">
              <svg className="w-16 h-16 mx-auto mb-4 text-green-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <h3 className="text-lg font-medium text-gray-900 mb-2">
                No Conflicts Found
              </h3>
              <p className="text-gray-600 mb-4">
                All your changes have been synchronized successfully.
              </p>
              <button
                onClick={onClose}
                className="px-4 py-2 bg-blue-600 text-white rounded-md hover:bg-blue-700 transition-colors"
              >
                Close
              </button>
            </div>
          ) : (
            <div className="space-y-6 max-h-96 overflow-y-auto">
              {conflicts.map((conflict, index) => (
                <ConflictItemComponent
                  key={conflict.operation.id}
                  conflict={conflict}
                  onResolve={handleResolve}
                />
              ))}
            </div>
          )}

          {conflicts.length > 0 && (
            <div className="mt-6 pt-4 border-t border-gray-200">
              <div className="flex items-center justify-between">
                <div className="text-sm text-gray-600">
                  <p>
                    Conflicts occur when the same data is modified both locally and on the server.
                  </p>
                  <p className="mt-1">
                    Choose which version to keep, or create a merged version with your preferred changes.
                  </p>
                </div>
                <div className="flex items-center space-x-3">
                  <button
                    onClick={onClose}
                    className="px-4 py-2 text-sm text-gray-700 bg-gray-200 hover:bg-gray-300 rounded-md transition-colors"
                  >
                    Close
                  </button>
                </div>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

export const SyncStatus: React.FC = () => {
  const [syncStats, setSyncStats] = useState<{
    pendingOperations: number;
    failedOperations: number;
    lastSyncTime: number | null;
    isOnline: boolean;
    isSyncing: boolean;
  }>({
    pendingOperations: 0,
    failedOperations: 0,
    lastSyncTime: null,
    isOnline: true,
    isSyncing: false,
  });
  const [showConflictResolver, setShowConflictResolver] = useState(false);

  const { getSyncStats, processPendingOperations, resolvePendingConflicts } = useSyncQueue();

  useEffect(() => {
    const updateStats = async () => {
      try {
        const stats = await getSyncStats();
        setSyncStats(stats);
      } catch (error) {
        console.error('Failed to get sync stats:', error);
      }
    };

    updateStats();
    const interval = setInterval(updateStats, 5000);
    return () => clearInterval(interval);
  }, [getSyncStats]);

  const handleShowConflicts = async () => {
    await resolvePendingConflicts();
    setShowConflictResolver(true);
  };

  const getStatusColor = () => {
    if (!syncStats.isOnline) return 'text-red-600';
    if (syncStats.isSyncing) return 'text-blue-600';
    if (syncStats.failedOperations > 0) return 'text-orange-600';
    if (syncStats.pendingOperations > 0) return 'text-yellow-600';
    return 'text-green-600';
  };

  const getStatusText = () => {
    if (!syncStats.isOnline) return 'Offline';
    if (syncStats.isSyncing) return 'Syncing...';
    if (syncStats.failedOperations > 0) return `${syncStats.failedOperations} failed`;
    if (syncStats.pendingOperations > 0) return `${syncStats.pendingOperations} pending`;
    return 'Synced';
  };

  const formatLastSync = (timestamp: number | null) => {
    if (!timestamp) return 'Never';
    const date = new Date(timestamp);
    const now = new Date();
    const diff = now.getTime() - date.getTime();
    
    if (diff < 60000) return 'Just now';
    if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
    return date.toLocaleString();
  };

  return (
    <>
      <div className="flex items-center space-x-2 text-sm">
        <div className={`flex items-center space-x-1 ${getStatusColor()}`}>
          {syncStats.isSyncing ? (
            <svg className="w-4 h-4 animate-spin" viewBox="0 0 24 24">
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
          ) : (
            <div className={`w-2 h-2 rounded-full ${
              !syncStats.isOnline ? 'bg-red-600' :
              syncStats.failedOperations > 0 ? 'bg-orange-600' :
              syncStats.pendingOperations > 0 ? 'bg-yellow-600' :
              'bg-green-600'
            }`} />
          )}
          <span className="font-medium">{getStatusText()}</span>
        </div>
        
        <span className="text-gray-400">•</span>
        <span className="text-gray-600">
          Last sync: {formatLastSync(syncStats.lastSyncTime)}
        </span>

        {syncStats.failedOperations > 0 && (
          <>
            <span className="text-gray-400">•</span>
            <button
              onClick={handleShowConflicts}
              className="text-orange-600 hover:text-orange-700 underline"
            >
              View conflicts
            </button>
          </>
        )}

        {syncStats.pendingOperations > 0 && syncStats.isOnline && (
          <>
            <span className="text-gray-400">•</span>
            <button
              onClick={() => processPendingOperations()}
              className="text-blue-600 hover:text-blue-700 underline"
            >
              Sync now
            </button>
          </>
        )}
      </div>

      <ConflictResolver
        isOpen={showConflictResolver}
        onClose={() => setShowConflictResolver(false)}
        onAllResolved={() => setShowConflictResolver(false)}
      />
    </>
  );
};