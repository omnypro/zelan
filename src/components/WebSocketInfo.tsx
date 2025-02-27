import React, { useState } from 'react';
import { WebSocketInfo as WebSocketInfoType } from '../types';

interface WebSocketInfoProps {
  wsInfo: WebSocketInfoType;
  onUpdatePort: (port: number) => Promise<void>;
  loading: boolean;
}

/**
 * Component displaying WebSocket configuration information
 */
const WebSocketInfo: React.FC<WebSocketInfoProps> = ({
  wsInfo,
  onUpdatePort,
  loading
}) => {
  const [newPort, setNewPort] = useState(wsInfo.port.toString());

  const handleUpdatePort = async () => {
    const port = parseInt(newPort, 10);
    
    if (isNaN(port) || port < 1024 || port > 65535) {
      alert('Port must be a number between 1024 and 65535');
      return;
    }

    await onUpdatePort(port);
  };

  return (
    <div className="websocket-info">
      <div className="websocket-connection">
        <h4>Event Stream Connection</h4>
        <p className="uri-display">
          <code>{wsInfo.uri}</code>
        </p>
        <div className="port-configuration">
          <div className="input-group">
            <label htmlFor="ws-port">Port:</label>
            <input
              id="ws-port"
              type="number"
              value={newPort}
              onChange={(e) => setNewPort(e.target.value)}
              min="1024"
              max="65535"
            />
            <button
              onClick={handleUpdatePort}
              disabled={loading || newPort === wsInfo.port.toString()}
              className="action-button small"
            >
              Update
            </button>
          </div>
          <p className="help-text">
            Change requires app restart to take effect
          </p>
        </div>
      </div>

      <div className="connection-help">
        <h4>Terminal Connection</h4>
        <p>Connect to the event stream using:</p>
        <pre className="terminal-command">{wsInfo.wscat}</pre>
        <p>Or with websocat:</p>
        <pre className="terminal-command">{wsInfo.websocat}</pre>
        <h4>HTTP API</h4>
        <p>REST API available at:</p>
        <pre className="terminal-command">{wsInfo.httpUri}</pre>
      </div>
    </div>
  );
};

export default WebSocketInfo;