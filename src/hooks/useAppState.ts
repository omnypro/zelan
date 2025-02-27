import { useReducer } from 'react';
import { AppState, AppAction } from '../types';

// Initial state
const initialState: AppState = {
  eventBusStats: null,
  adapterStatuses: null,
  adapterSettings: null,
  testEventResult: '',
  wsInfo: null,
  activeTab: 'dashboard',
  loading: false,
  errors: [],
  lastUpdated: null,
  newPort: '',
  editingAdapterConfig: null,
};

// Reducer function to handle state updates
const appReducer = (state: AppState, action: AppAction): AppState => {
  switch (action.type) {
    case 'SET_EVENT_BUS_STATS':
      return { ...state, eventBusStats: action.payload };
    
    case 'SET_ADAPTER_STATUSES':
      return { ...state, adapterStatuses: action.payload };
    
    case 'SET_ADAPTER_SETTINGS':
      return { ...state, adapterSettings: action.payload };
    
    case 'SET_TEST_EVENT_RESULT':
      return { ...state, testEventResult: action.payload };
    
    case 'SET_WS_INFO':
      return { 
        ...state, 
        wsInfo: action.payload,
        // Initialize newPort from wsInfo if it's empty
        newPort: state.newPort === '' ? action.payload.port.toString() : state.newPort
      };
    
    case 'SET_ACTIVE_TAB':
      return { ...state, activeTab: action.payload };
    
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
    
    case 'ADD_ERROR':
      return { 
        ...state, 
        errors: [action.payload, ...state.errors].slice(0, 5) // Keep only the 5 most recent errors
      };
    
    case 'DISMISS_ERROR':
      return {
        ...state,
        errors: state.errors.filter((_, i) => i !== action.payload)
      };
    
    case 'SET_LAST_UPDATED':
      return { ...state, lastUpdated: action.payload };
    
    case 'SET_NEW_PORT':
      return { ...state, newPort: action.payload };
    
    case 'SET_EDITING_ADAPTER_CONFIG':
      return { ...state, editingAdapterConfig: action.payload };
    
    case 'CLEAR_TEST_EVENT_RESULT':
      return { ...state, testEventResult: '' };
      
    default:
      return state;
  }
};

// Custom hook to provide state and dispatch
export const useAppState = () => {
  const [state, dispatch] = useReducer(appReducer, initialState);
  
  return { state, dispatch };
};