import React, { useState } from 'react';
import { AdapterStatus } from '../../../shared/adapters/interfaces/AdapterStatus';

export interface Adapter {
  id: string;
  type: string;
  name: string;
  enabled: boolean;
  status?: {
    status?: AdapterStatus;
    message?: string;
  };
  options: Record<string, any>;
}

interface AdapterListProps {
  adapters: Adapter[];
  loadingAdapterId?: string | null;
  onStartAdapter: (id: string) => Promise<void>;
  onStopAdapter: (id: string) => Promise<void>;
  onDeleteAdapter: (id: string) => Promise<void>;
}

const AdapterList: React.FC<AdapterListProps> = ({
  adapters,
  loadingAdapterId,
  onStartAdapter,
  onStopAdapter,
  onDeleteAdapter
}) => {
  const [error, setError] = useState<string | null>(null);

  const handleStartAdapter = async (id: string) => {
    setError(null);
    try {
      await onStartAdapter(id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to start adapter');
      console.error('Error starting adapter:', err);
    }
  };

  const handleStopAdapter = async (id: string) => {
    setError(null);
    try {
      await onStopAdapter(id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to stop adapter');
      console.error('Error stopping adapter:', err);
    }
  };

  const handleDeleteAdapter = async (id: string) => {
    if (!window.confirm('Are you sure you want to delete this adapter?')) {
      return;
    }
    
    setError(null);
    try {
      await onDeleteAdapter(id);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to delete adapter');
      console.error('Error deleting adapter:', err);
    }
  };

  const getStatusClass = (status?: AdapterStatus) => {
    switch (status) {
      case AdapterStatus.CONNECTED:
        return 'bg-green-100 text-green-800';
      case AdapterStatus.CONNECTING:
      case AdapterStatus.RECONNECTING:
        return 'bg-yellow-100 text-yellow-800';
      case AdapterStatus.ERROR:
        return 'bg-red-100 text-red-800';
      case AdapterStatus.DISCONNECTED:
      default:
        return 'bg-gray-100 text-gray-800';
    }
  };

  if (adapters.length === 0) {
    return (
      <div className="p-4 border rounded-lg shadow-sm bg-white">
        <p className="text-center text-gray-500">No adapters found</p>
      </div>
    );
  }

  return (
    <div className="bg-white rounded-lg shadow">
      {error && (
        <div className="p-4 bg-red-100 border-b border-red-200 text-red-700 rounded-t-lg">
          <p>{error}</p>
        </div>
      )}

      <ul className="divide-y divide-gray-200">
        {adapters.map((adapter) => (
          <li key={adapter.id} className="p-4">
            <div className="flex flex-col sm:flex-row sm:items-center justify-between">
              <div className="mb-3 sm:mb-0">
                <div className="flex items-center">
                  <span className="font-medium text-lg">{adapter.name}</span>
                  <span className={`ml-2 text-xs px-2 py-1 rounded ${getStatusClass(adapter.status?.status)}`}>
                    {adapter.status?.status || 'unknown'}
                  </span>
                  <span className="ml-2 text-xs bg-gray-100 text-gray-800 px-2 py-1 rounded">
                    {adapter.type}
                  </span>
                </div>
                {adapter.status?.message && (
                  <p className="text-sm text-gray-500 mt-1">{adapter.status.message}</p>
                )}
              </div>
              
              <div className="flex space-x-2">
                {adapter.status?.status === AdapterStatus.CONNECTED ? (
                  <button
                    onClick={() => handleStopAdapter(adapter.id)}
                    disabled={loadingAdapterId === adapter.id}
                    className="px-3 py-1 bg-red-600 text-white rounded-md hover:bg-red-700 focus:outline-none focus:ring-2 focus:ring-red-500 disabled:opacity-50"
                  >
                    {loadingAdapterId === adapter.id ? 'Stopping...' : 'Stop'}
                  </button>
                ) : (
                  <button
                    onClick={() => handleStartAdapter(adapter.id)}
                    disabled={
                      loadingAdapterId === adapter.id || 
                      adapter.status?.status === AdapterStatus.CONNECTING || 
                      adapter.status?.status === AdapterStatus.RECONNECTING
                    }
                    className="px-3 py-1 bg-green-600 text-white rounded-md hover:bg-green-700 focus:outline-none focus:ring-2 focus:ring-green-500 disabled:opacity-50"
                  >
                    {loadingAdapterId === adapter.id 
                      ? 'Starting...' 
                      : adapter.status?.status === AdapterStatus.CONNECTING || adapter.status?.status === AdapterStatus.RECONNECTING
                        ? 'Connecting...'
                        : 'Start'
                    }
                  </button>
                )}
                
                <button
                  onClick={() => handleDeleteAdapter(adapter.id)}
                  disabled={loadingAdapterId === adapter.id}
                  className="px-3 py-1 bg-gray-200 text-gray-800 rounded-md hover:bg-gray-300 focus:outline-none focus:ring-2 focus:ring-gray-400 disabled:opacity-50"
                >
                  Delete
                </button>
              </div>
            </div>
            
            <div className="mt-3">
              <div className="text-sm">
                <span className="font-medium">ID: </span>
                <span className="text-gray-600">{adapter.id}</span>
              </div>
              
              <div className="mt-2">
                <details className="text-sm">
                  <summary className="cursor-pointer font-medium text-blue-600 hover:text-blue-800">
                    Show Configuration
                  </summary>
                  <pre className="mt-2 p-2 bg-gray-50 rounded text-xs overflow-auto max-h-60">
                    {JSON.stringify(adapter.options, null, 2)}
                  </pre>
                </details>
              </div>
            </div>
          </li>
        ))}
      </ul>
    </div>
  );
};

export default AdapterList;