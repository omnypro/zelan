import React from 'react';
import { EventBusStats as EventBusStatsType } from '../types';

interface EventBusStatsProps {
  stats: EventBusStatsType;
  lastUpdated: Date | null;
}

/**
 * Component for displaying event bus statistics in a desktop-friendly way
 */
const EventBusStats: React.FC<EventBusStatsProps> = ({ 
  stats, 
  lastUpdated 
}) => {
  return (
    <div>
      <div className="panel-header">
        <h3>Event Bus Statistics</h3>
        {lastUpdated && (
          <span className="last-updated">
            Last updated: {lastUpdated.toLocaleTimeString()}
          </span>
        )}
      </div>

      <div className="stat-summary">
        <div className="stat-box">
          <span className="stat-value">
            {stats.events_published}
          </span>
          <span className="stat-label">Events Published</span>
        </div>
        <div className="stat-box">
          <span className="stat-value">
            {stats.events_dropped}
          </span>
          <span className="stat-label">Events Dropped</span>
        </div>
      </div>

      <h4>Source Counts</h4>
      <ul className="source-list">
        {Object.entries(stats.source_counts || {}).map(
          ([source, count]) => (
            <li key={source} className="source-item">
              <span className="source-name">{source}</span>
              <span className="source-count">{count as number}</span>
            </li>
          )
        )}
        {Object.keys(stats.source_counts || {}).length === 0 && 
          <li className="empty-list">No events recorded yet</li>}
      </ul>

      <h4>Event Types</h4>
      <ul className="type-list">
        {Object.entries(stats.type_counts || {}).map(
          ([type, count]) => (
            <li key={type} className="type-item">
              <span className="type-name">{type}</span>
              <span className="type-count">{count as number}</span>
            </li>
          )
        )}
        {Object.keys(stats.type_counts || {}).length === 0 && 
          <li className="empty-list">No events recorded yet</li>}
      </ul>
    </div>
  );
};

export default EventBusStats;