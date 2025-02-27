import React from 'react';
import { ZelanError } from '../types';

interface ErrorNotificationProps {
  error: ZelanError | string;
  onDismiss: () => void;
}

/**
 * Desktop-style notification component for displaying errors and informational messages
 */
const ErrorNotification: React.FC<ErrorNotificationProps> = ({ error, onDismiss }) => {
  // Handle both string errors and ZelanError objects
  const errorObj =
    typeof error === 'string'
      ? {
          code: 'UNKNOWN',
          message: error,
          severity: 'error' as const,
        }
      : error;

  // Map severity to CSS class
  const severityClass =
    {
      info: 'info',
      warning: 'warning',
      error: 'error',
      critical: 'critical',
    }[errorObj.severity] || 'error';

  return (
    <div className={`error-notification ${severityClass}`}>
      <div className="error-header">
        <span className="error-code">{errorObj.code}</span>
        <button className="dismiss-button" onClick={onDismiss}>
          ×
        </button>
      </div>
      <p className="error-message">{errorObj.message}</p>
      {errorObj.context && <p className="error-context">{errorObj.context}</p>}
    </div>
  );
};

export default ErrorNotification;