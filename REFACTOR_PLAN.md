# Zelan Refactor Plan

## Implemented Enhancements

### 1. Enhanced Error Recovery System

#### Implemented:
- Created a comprehensive error classification system with categories (Network, Authentication, RateLimit, etc.)
- Implemented RetryPolicy for automatic retries with exponential backoff and jitter
- Added CircuitBreaker pattern to prevent cascading failures
- Created ErrorRegistry for centralized error tracking and analysis
- Added RecoveryManager to coordinate error recovery strategies
- Implemented AdapterRecovery trait for adapter-specific recovery strategies
- Integrated with TwitchAdapter for robust error handling

#### Benefits:
- Improved resilience against transient errors
- Better handling of rate limits and timeouts
- More detailed error logging and categorization
- Centralized error tracking for debugging
- Automatic retry with intelligent backoff for different error types
- Circuit breaker protection against persistent failures

## Phase 1 Implementation Plan

### Timeline
- **Week 1**: Rust backend simplifications
- **Week 2**: TypeScript frontend refactoring
- **Week 3**: Integration testing and cleanup

### Tasks and Priorities

#### 1. Rust Backend Improvements (Estimated: 5 days)

1. **Error Handling System (2 days)**
   - [X] Implement error builder pattern
   - [X] Convert ErrorRegistry to use VecDeque
   - [X] Add thiserror dependency and evaluate conversion

2. **CircuitBreaker Simplification (1 day)**
   - [X] Evaluate external library integration (decided against it)
   - [X] Implement atomic types for state management
   - [X] Create SimpleCircuitBreaker with better API

3. **Twitch Adapter Refactoring (2 days)**
   - [ ] Extract authentication to separate TwitchAuthenticator
   - [ ] Simplify token recovery logic
   - [ ] Streamline event handling

#### 2. TypeScript Frontend Improvements (Estimated: 5 days)

1. **Component Structure (2 days)**
   - [ ] Split App.tsx into Dashboard and Settings components
   - [ ] Create ErrorNotification and StatusIndicator components
   - [ ] Implement WebSocketInfo component

2. **State Management (1 day)**
   - [ ] Convert to useReducer pattern
   - [ ] Create action creators and types

3. **Type Improvements (1 day)**
   - [ ] Create proper EventBusStats interface
   - [ ] Define adapter configuration types
   - [ ] Remove any usage from codebase

4. **Custom Hooks (1 day)**
   - [ ] Implement useTauriCommand
   - [ ] Create useAdapterControl
   - [ ] Add useWebSocketStatus

#### 3. Integration and Testing (Estimated: 5 days)

1. **Test Implementation (3 days)**
   - [ ] Add unit tests for error handling
   - [ ] Create tests for CircuitBreaker
   - [ ] Add integration tests for authentication flow

2. **Documentation and Cleanup (2 days)**
   - [ ] Update code comments and documentation
   - [ ] Create examples for using the simplified APIs
   - [ ] Clean up unused code and imports

## Completed Enhancements

### 2. Codebase Simplification - Phase 1

#### 2.1 Error Handling System Improvements
- **Implemented error builder pattern**:
  ```rust
  // Instead of direct struct creation:
  ZelanError {
      code: ErrorCode::AdapterNotFound,
      message: format!("Adapter '{}' not found", name),
      context: None,
      severity: ErrorSeverity::Error,
      category: Some(ErrorCategory::NotFound),
      error_id: None,
  }
  
  // Now using builder pattern:
  ZelanError::new(ErrorCode::AdapterNotFound)
      .message(format!("Adapter '{}' not found", name))
      .category(ErrorCategory::NotFound)
      .severity(ErrorSeverity::Error)
      .build()
  ```

- **Improved error registry with VecDeque for proper FIFO**:
  ```rust
  // Previously:
  error_history: RwLock<HashMap<String, ZelanError>>
  
  // Now:
  error_history: RwLock<VecDeque<ZelanError>>,
  error_id_map: RwLock<HashMap<String, usize>>,
  ```

- **Added thiserror integration**:
  ```rust
  #[derive(Error, Debug, Clone)]
  pub enum ZelanErrorType {
      #[error("Adapter '{name}' not found")]
      AdapterNotFound { name: String },
      
      #[error("Failed to connect adapter '{name}': {reason}")]
      AdapterConnectionFailed { name: String, reason: String },
      
      // Other variants...
  }
  ```

#### 2.2 Circuit Breaker Improvements
- **Implemented improved CircuitBreaker with atomic types**:
  ```rust
  pub struct SimpleCircuitBreaker {
      name: String,
      failure_threshold: u32,
      reset_timeout: Duration,
      is_open: Arc<AtomicBool>,
      failure_count: Arc<AtomicUsize>,
      open_time: Arc<RwLock<Option<Instant>>>,
      error_registry: Option<Arc<ErrorRegistry>>,
  }
  ```

- **Created simplified recovery system**:
  ```rust
  pub struct SimpleRecoveryManager {
      error_registry: Arc<ErrorRegistry>,
      circuit_breakers: Arc<RwLock<HashMap<String, SimpleCircuitBreaker>>>,
      default_policies: Arc<HashMap<ErrorCategory, RetryPolicy>>,
  }
  
  pub trait SimpleAdapterRecovery {
      fn recovery_manager(&self) -> Arc<SimpleRecoveryManager>;
      fn adapter_name(&self) -> &str;
      // Plus convenience methods...
  }
  ```

##### 2.3 Twitch Adapter Simplification
- **Extract authentication into a separate component**:
  ```rust
  pub struct TwitchAuthenticator {
      auth_manager: Arc<RwLock<TwitchAuthManager>>,
      token_manager: Option<Arc<TokenManager>>,
      on_auth_event: Option<Box<dyn Fn(AuthEvent) -> Result<()> + Send + Sync>>,
  }
  
  impl TwitchAuthenticator {
      // Authentication methods...
  }
  ```

- **Simplify token recovery logic**:
  ```rust
  impl TwitchAdapter {
      async fn ensure_authenticated(&self) -> Result<Token> {
          // Single method handling all authentication scenarios
      }
  }
  ```

##### 2.4 Recovery System Improvements
- **Consolidate retry logic**:
  ```rust
  // In recovery.rs
  impl RecoveryManager {
      pub async fn retry<F, Fut, T>(&self, operation: Operation<F, T>) -> ZelanResult<T>
      where
          F: Fn() -> Fut + Send + Sync + Clone,
          Fut: std::future::Future<Output = ZelanResult<T>> + Send,
      {
          // Single implementation of retry logic with circuit breaker integration
      }
  }
  ```

- **Consider `tokio-retry` for retry implementation**:
  ```rust
  use tokio_retry::{Retry, RetryIf};
  use tokio_retry::strategy::{ExponentialBackoff, FixedInterval};
  
  // Replace custom retry logic
  pub async fn with_retry<F, Fut, T>(
      operation_name: &str,
      policy: &RetryPolicy,
      f: F,
  ) -> ZelanResult<T>
  where
      F: Fn() -> Fut + Send + Sync,
      Fut: std::future::Future<Output = ZelanResult<T>> + Send,
  {
      // Implementation using tokio-retry
  }
  ```

#### TypeScript/Frontend Improvements

##### 2.5 Component Structure
- **Split App.tsx into smaller components**:
  ```typescript
  // Extract to components/Dashboard.tsx
  function Dashboard({ wsInfo, eventBusStats, adapterStatuses }) { ... }
  
  // Extract to components/Settings.tsx
  function Settings({ adapterSettings, adapterStatuses }) { ... }
  ```

##### 2.6 State Management
- **Replace multiple useState calls with useReducer**:
  ```typescript
  type AppState = {
    eventBusStats: EventBusStats | null;
    adapterStatuses: AdapterStatusMap | null;
    adapterSettings: AdapterSettingsMap | null;
    wsInfo: WebSocketInfo | null;
    loading: boolean;
  };

  function appReducer(state: AppState, action: AppAction): AppState {
    switch (action.type) {
      case 'SET_DATA':
        return { ...state, ...action.payload, loading: false };
      case 'SET_LOADING':
        return { ...state, loading: action.payload };
      // other cases
    }
  }
  ```

##### 2.7 Custom Hooks
- **Create a `useTauriCommand` hook**:
  ```typescript
  function useTauriCommand<T>() {
    const [loading, setLoading] = useState(false);
    const [error, setError] = useState<ZelanError | null>(null);
    
    const execute = useCallback(async (command: string, args?: any): Promise<T | null> => {
      try {
        setLoading(true);
        const result = await invoke<T>(command, args);
        return result;
      } catch (err) {
        setError(err as ZelanError);
        return null;
      } finally {
        setLoading(false);
      }
    }, []);
    
    return { execute, loading, error };
  }
  ```

##### 2.8 Type Improvements
- **Replace `any` with proper types**:
  ```typescript
  interface EventBusStats {
    events_published: number;
    events_dropped: number;
    source_counts: Record<string, number>;
    type_counts: Record<string, number>;
  }
  
  // For adapter configs
  interface TwitchAdapterConfig {
    channel_id?: string;
    channel_login?: string;
    access_token?: string | null;
    refresh_token?: string | null;
  }
  ```

### 3. Comprehensive Testing Suite

#### Plan:
- Create unit tests for all adapter components
- Add integration tests for end-to-end flows
- Implement mocks for external APIs (Twitch, OBS)
- Add automated testing for authentication flows
- Create a CI pipeline for continuous testing

### 4. Enhanced WebSocket Client Support

#### Plan:
- Develop a JavaScript client library for web overlays
- Create a simple Python client for integration with other tools
- Add TypeScript types for all events
- Implement reconnection logic in clients
- Add examples of common integration patterns
