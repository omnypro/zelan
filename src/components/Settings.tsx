import React from 'react';
import { AdapterSettingsMap, AdapterStatusMap } from '../types';
import AdapterCard from './AdapterCard';
import ConfigurationForm from './ConfigurationForm';

interface SettingsProps {
  adapterSettings: AdapterSettingsMap | null;
  adapterStatuses: AdapterStatusMap | null;
  loading: boolean;
  editingAdapterConfig: string | null;
  onToggleAdapter: (adapterName: string) => void;
  onConfigureAdapter: (adapterName: string) => void;
  onCloseConfigModal: () => void;
  onSaveAdapterConfig: (adapterName: string, configUpdates: Record<string, any>) => void;
}

/**
 * Settings component for adapter configuration
 */
const Settings: React.FC<SettingsProps> = ({
  adapterSettings,
  adapterStatuses,
  loading,
  editingAdapterConfig,
  onToggleAdapter,
  onConfigureAdapter,
  onCloseConfigModal,
  onSaveAdapterConfig,
}) => {
  return (
    <div className="settings-section">
      <h2>Adapter Configuration</h2>
      <p className="settings-description">
        Enable, disable, and configure data adapters. Changes to adapter
        status will take effect immediately.
      </p>

      {loading ? (
        <p className="loading">Loading adapter settings...</p>
      ) : (
        <div className="adapters-grid">
          {adapterSettings &&
            Object.entries(adapterSettings).map(
              ([adapterName, settings]) => {
                const status =
                  adapterStatuses?.[adapterName] || 'Disconnected';
                return (
                  <AdapterCard
                    key={adapterName}
                    adapterName={adapterName}
                    settings={settings}
                    status={status}
                    onToggle={onToggleAdapter}
                    onConfigure={onConfigureAdapter}
                  />
                );
              }
            )}

          {(!adapterSettings ||
            Object.keys(adapterSettings).length === 0) && (
            <p className="no-adapters">No adapters found</p>
          )}
        </div>
      )}

      {/* Configuration Modal */}
      {editingAdapterConfig && adapterSettings && (
        <div className="modal-overlay">
          <div className="modal">
            <div className="modal-header">
              <h3>
                Configure {adapterSettings[editingAdapterConfig]?.display_name}
              </h3>
              <button
                className="modal-close"
                onClick={onCloseConfigModal}
              >
                ×
              </button>
            </div>

            <div className="modal-content">
              <ConfigurationForm
                adapterName={editingAdapterConfig}
                config={adapterSettings[editingAdapterConfig]?.config || {}}
                onSave={(configUpdates) => {
                  onSaveAdapterConfig(editingAdapterConfig, configUpdates);
                }}
                onCancel={onCloseConfigModal}
              />
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

export default Settings;