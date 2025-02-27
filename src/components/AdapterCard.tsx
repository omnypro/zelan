import React from 'react';
import { AdapterSettings, ServiceStatus } from '../types';
import StatusIndicator from './StatusIndicator';

interface AdapterCardProps {
  adapterName: string;
  settings: AdapterSettings;
  status: ServiceStatus;
  onToggle: (adapterName: string) => void;
  onConfigure: (adapterName: string) => void;
}

/**
 * Card component for displaying an adapter with its configuration and status
 */
const AdapterCard: React.FC<AdapterCardProps> = ({
  adapterName,
  settings,
  status,
  onToggle,
  onConfigure,
}) => {
  return (
    <div className={`adapter-card ${settings.enabled ? '' : 'disabled'}`}>
      <div className="adapter-header">
        <h3>{settings.display_name}</h3>
        <StatusIndicator status={status} />
      </div>

      <p className="adapter-description">
        {settings.description}
      </p>

      <div className="adapter-config">
        {Object.entries(settings.config || {}).map(
          ([key, value]) => (
            <div key={key} className="config-item">
              <span className="config-label">{key}:</span>
              <span className="config-value">
                {typeof value === 'boolean'
                  ? value
                    ? 'Yes'
                    : 'No'
                  : String(value)}
              </span>
            </div>
          )
        )}
      </div>

      <div className="adapter-actions">
        <button
          className={`toggle-button ${
            settings.enabled ? 'enabled' : 'disabled'
          }`}
          onClick={() => onToggle(adapterName)}
        >
          {settings.enabled ? 'Disable' : 'Enable'}
        </button>

        <button
          className="config-button"
          onClick={() => onConfigure(adapterName)}
          disabled={!settings.enabled}
        >
          Configure
        </button>
      </div>
    </div>
  );
};

export default AdapterCard;