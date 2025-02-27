import React from 'react';
import { AdapterStatusMap } from '../types';
import StatusIndicator from './StatusIndicator';

interface AdapterStatusListProps {
  adapterStatuses: AdapterStatusMap;
}

/**
 * Component displaying a list of all adapters and their current status
 */
const AdapterStatusList: React.FC<AdapterStatusListProps> = ({ 
  adapterStatuses 
}) => {
  return (
    <div>
      <div className="panel-header">
        <h3>Adapter Status</h3>
      </div>

      <ul className="adapter-list">
        {Object.entries(adapterStatuses).map(([adapter, status]) => {
          return (
            <li key={adapter} className={`adapter-item`}>
              <span className="adapter-name">{adapter}</span>
              <StatusIndicator 
                status={status} 
                showLabel={true}
                size="small"
              />
            </li>
          );
        })}
        {Object.keys(adapterStatuses).length === 0 && (
          <li className="empty-list">No adapters registered</li>
        )}
      </ul>
    </div>
  );
};

export default AdapterStatusList;