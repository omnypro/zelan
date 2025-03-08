/**
 * OBS specific event types
 */
export enum ObsEventType {
  SCENE_CHANGED = 'scene_changed',
  SCENE_SWITCHED = 'scene_switched',
  SCENE_LIST_CHANGED = 'scene_list_changed',
  STREAM_STARTED = 'stream_started',
  STREAM_STOPPED = 'stream_stopped',
  STREAMING_STARTED = 'streaming_started',
  STREAMING_STOPPED = 'streaming_stopped',
  RECORDING_STARTED = 'recording_started',
  RECORDING_STOPPED = 'recording_stopped',
  RECORDING_PAUSED = 'recording_paused',
  RECORDING_RESUMED = 'recording_resumed',
  VIRTUAL_CAM_STARTED = 'virtual_cam_started',
  VIRTUAL_CAM_STOPPED = 'virtual_cam_stopped',
  SOURCE_VISIBILITY_CHANGED = 'source_visibility_changed',
  SOURCE_CHANGED = 'source_changed',
  SCENE_COLLECTION_CHANGED = 'scene_collection_changed'
}

/**
 * OBS Scene change event payload
 */
export interface ObsSceneChangePayload {
  sceneName: string;
  previousSceneName?: string;
  transitionName?: string;
  transitionDuration?: number;
  sceneItems?: Array<{
    id: number;
    name: string;
    visible: boolean;
  }>;
}

/**
 * OBS Stream state event payload
 */
export interface ObsStreamStatePayload {
  active: boolean;
  startTime?: number;
  duration?: number;
  kbitsPerSec?: number;
  numDroppedFrames?: number;
  numTotalFrames?: number;
}

/**
 * OBS Recording state event payload
 */
export interface ObsRecordStatePayload {
  active: boolean;
  startTime?: number;
  duration?: number;
  filePath?: string;
  paused?: boolean;
}

/**
 * OBS Source visibility event payload
 */
export interface ObsSourceVisibilityPayload {
  sceneName: string;
  sourceName: string;
  sourceId: number; 
  visible: boolean;
}