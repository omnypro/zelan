import React from 'react';
import { ServiceStatus } from '../types';

interface StatusIndicatorProps {
  status: ServiceStatus;
  label?: string;
  showLabel?: boolean;
  size?: 'small' | 'medium' | 'large';
}

/**
 * A desktop-style status indicator that shows connection status with appropriate colors
 */
const StatusIndicator: React.FC<StatusIndicatorProps> = ({
  status,
  label,
  showLabel = true,
  size = 'medium'
}) => {
  // Map status to CSS class
  const statusClass = 
    {
      Connected: 'status-connected',
      Connecting: 'status-connecting',
      Disconnected: 'status-disconnected',
      Error: 'status-error',
      Disabled: 'status-disabled'
    }[status] || 'status-disconnected';

  return (
    <div className={`status-indicator ${statusClass} size-${size}`}>
      <div className="status-dot"></div>
      {showLabel && (
        <span className="status-label">
          {label || status}
        </span>
      )}
    </div>
  );
};

export default StatusIndicator;