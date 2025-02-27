import React, { useState } from 'react';

interface ConfigurationFormProps {
  adapterName: string;
  config: Record<string, any>;
  onSave: (config: Record<string, any>) => void;
  onCancel: () => void;
}

/**
 * Form component for adapter configuration
 */
const ConfigurationForm: React.FC<ConfigurationFormProps> = ({
  adapterName,
  config,
  onSave,
  onCancel,
}) => {
  const [formState, setFormState] = useState<Record<string, any>>(config);
  const hasToken = Boolean(config.access_token);

  // Handle input changes
  const handleInputChange = (key: string, value: any) => {
    setFormState({
      ...formState,
      [key]: value,
    });
  };

  return (
    <div>
      {/* If Twitch adapter, show authentication info */}
      {adapterName === 'twitch' && (
        <div className="auth-section">
          {/* Show requirements notice */}
          <div className="auth-requirements">
            <p className="note">
              <strong>Note:</strong> For Twitch integration to work properly, you'll need to set 
              either a Channel ID (numeric) or Channel Login (username) below. These are used 
              after authentication to fetch channel and stream information.
            </p>
          </div>
          
          <TwitchAuthInfo hasToken={hasToken} />
          
          {hasToken && (
            <div className="auth-actions">
              <button 
                className="action-button small"
                onClick={() => {
                  // Clear tokens and reset auth status
                  const updatedConfig = {
                    ...formState,
                    access_token: null,
                    refresh_token: null,
                  };
                  setFormState(updatedConfig);
                }}
              >
                Disconnect
              </button>
            </div>
          )}
          
          <hr className="section-divider" />
        </div>
      )}
      
      <form
        onSubmit={(e) => {
          e.preventDefault();
          onSave(formState);
        }}
      >
        {Object.entries(config).map(([key, value]) => {
          // For Twitch adapter, hide token fields from editing
          if (adapterName === 'twitch' && (key === 'access_token' || key === 'refresh_token')) {
            return null;
          }
        
          // Render appropriate input based on value type
          if (typeof value === 'boolean') {
            return (
              <div key={key} className="form-group">
                <label>{key}:</label>
                <select
                  value={formState[key]?.toString() || 'false'}
                  onChange={(e) =>
                    handleInputChange(key, e.target.value === 'true')
                  }
                >
                  <option value="true">Yes</option>
                  <option value="false">No</option>
                </select>
              </div>
            );
          } else if (typeof value === 'number') {
            return (
              <div key={key} className="form-group">
                <label>{key}:</label>
                <input
                  type="number"
                  value={formState[key] || 0}
                  onChange={(e) => handleInputChange(key, Number(e.target.value))}
                />
              </div>
            );
          } else {
            return (
              <div key={key} className="form-group">
                <label>{key}:</label>
                <input
                  type="text"
                  value={formState[key] || ''}
                  onChange={(e) => handleInputChange(key, e.target.value)}
                />
              </div>
            );
          }
        })}

        <div className="form-actions">
          <button type="button" className="cancel-button" onClick={onCancel}>
            Cancel
          </button>
          <button type="submit" className="save-button">
            Save Changes
          </button>
        </div>
      </form>
    </div>
  );
};

/**
 * Simple information component for Twitch authentication
 */
const TwitchAuthInfo: React.FC<{ hasToken: boolean }> = ({ hasToken }) => {
  return (
    <div className="twitch-auth-container">
      <h3>Twitch Authentication</h3>
      
      {hasToken ? (
        <div className="auth-success">
          <p>✓ Successfully authenticated with Twitch!</p>
        </div>
      ) : (
        <div className="auth-instructions">
          <p>To authenticate with Twitch:</p>
          <ol>
            <li>Set your Channel ID and Channel Login below</li>
            <li>Enable the Twitch adapter using the toggle below</li>
            <li>Watch the terminal for authentication instructions</li>
            <li>Visit the URL shown in the terminal and enter the code</li>
            <li>The adapter will automatically connect once authenticated</li>
          </ol>
        </div>
      )}
    </div>
  );
};

export default ConfigurationForm;