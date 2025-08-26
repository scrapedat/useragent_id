use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use crate::types::PlannerError;

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Circuit is closed, requests flow normally
    Closed,
    /// Circuit is open, requests are rejected
    Open,
    /// Circuit is in half-open state, allowing a test request
    HalfOpen,
}

/// Configuration for the circuit breaker
#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to trip the circuit
    pub failure_threshold: usize,
    /// Reset timeout in milliseconds
    pub reset_timeout_ms: u64,
    /// Half-open request limit
    pub half_open_limit: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            reset_timeout_ms: 30000, // 30 seconds
            half_open_limit: 1,
        }
    }
}

/// Circuit breaker implementation for LaVague API
pub struct CircuitBreaker {
    /// Current state of the circuit
    state: RwLock<CircuitState>,
    /// Failure counter
    failure_counter: AtomicUsize,
    /// Last failure time
    last_failure: RwLock<Option<Instant>>,
    /// Configuration
    config: CircuitBreakerConfig,
    /// Counter for half-open requests
    half_open_counter: AtomicUsize,
}

impl CircuitBreaker {
    /// Create a new circuit breaker with the given configuration
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            state: RwLock::new(CircuitState::Closed),
            failure_counter: AtomicUsize::new(0),
            last_failure: RwLock::new(None),
            config,
            half_open_counter: AtomicUsize::new(0),
        }
    }
    
    /// Check if the circuit allows a request
    pub async fn allow_request(&self) -> Result<(), PlannerError> {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => {
                // Closed circuit allows all requests
                Ok(())
            },
            CircuitState::Open => {
                // Check if it's time to transition to half-open
                let last_failure = self.last_failure.read().await;
                if let Some(time) = *last_failure {
                    let elapsed = time.elapsed();
                    if elapsed >= Duration::from_millis(self.config.reset_timeout_ms) {
                        // Transition to half-open
                        *self.state.write().await = CircuitState::HalfOpen;
                        self.half_open_counter.store(0, Ordering::SeqCst);
                        return Ok(());
                    }
                }
                
                // Still open, reject the request
                Err(PlannerError::ServiceUnavailable("Circuit breaker is open".to_string()))
            },
            CircuitState::HalfOpen => {
                // Allow a limited number of requests in half-open state
                let current = self.half_open_counter.fetch_add(1, Ordering::SeqCst);
                if current < self.config.half_open_limit {
                    Ok(())
                } else {
                    Err(PlannerError::ServiceUnavailable("Circuit breaker is half-open and at capacity".to_string()))
                }
            }
        }
    }
    
    /// Record a successful request
    pub async fn on_success(&self) {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => {
                // Reset failure counter
                self.failure_counter.store(0, Ordering::SeqCst);
            },
            CircuitState::HalfOpen => {
                // Transition to closed on successful half-open request
                *self.state.write().await = CircuitState::Closed;
                self.failure_counter.store(0, Ordering::SeqCst);
                log::info!("Circuit breaker transitioned from half-open to closed");
            },
            CircuitState::Open => {
                // This shouldn't happen, but just in case
                log::warn!("Received success while circuit was open");
            }
        }
    }
    
    /// Record a failed request
    pub async fn on_failure(&self) {
        let state = *self.state.read().await;
        
        match state {
            CircuitState::Closed => {
                // Increment failure counter
                let failures = self.failure_counter.fetch_add(1, Ordering::SeqCst) + 1;
                
                // Check if we need to trip the circuit
                if failures >= self.config.failure_threshold {
                    *self.state.write().await = CircuitState::Open;
                    *self.last_failure.write().await = Some(Instant::now());
                    log::warn!("Circuit breaker tripped open after {} failures", failures);
                }
            },
            CircuitState::HalfOpen => {
                // Failed test request, back to open
                *self.state.write().await = CircuitState::Open;
                *self.last_failure.write().await = Some(Instant::now());
                log::warn!("Circuit breaker returned to open state after failed test request");
            },
            CircuitState::Open => {
                // Update last failure time
                *self.last_failure.write().await = Some(Instant::now());
            }
        }
    }
    
    /// Get the current state of the circuit
    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
    
    /// Force the circuit into a specific state (for testing)
    #[cfg(test)]
    pub async fn force_state(&self, state: CircuitState) {
        *self.state.write().await = state;
    }
}

/// Wrapper for circuit-protected function execution
pub struct CircuitProtected<T> {
    circuit_breaker: Arc<CircuitBreaker>,
    inner: T,
}

impl<T> CircuitProtected<T> {
    /// Create a new circuit-protected wrapper
    pub fn new(inner: T, circuit_breaker: Arc<CircuitBreaker>) -> Self {
        Self {
            circuit_breaker,
            inner,
        }
    }
    
    /// Get a reference to the inner value
    pub fn inner(&self) -> &T {
        &self.inner
    }
    
    /// Get a reference to the circuit breaker
    pub fn circuit_breaker(&self) -> &CircuitBreaker {
        &self.circuit_breaker
    }
    
    /// Execute a function with circuit breaker protection
    pub async fn execute<F, Fut, R, E>(&self, f: F) -> Result<R, PlannerError>
    where
        F: FnOnce(&T) -> Fut,
        Fut: std::future::Future<Output = Result<R, E>>,
        E: std::fmt::Display,
    {
        // Check if the circuit allows the request
        self.circuit_breaker.allow_request().await?;
        
        // Execute the function
        match f(&self.inner).await {
            Ok(result) => {
                // Record success
                self.circuit_breaker.on_success().await;
                Ok(result)
            },
            Err(e) => {
                // Record failure
                self.circuit_breaker.on_failure().await;
                Err(PlannerError::ServiceUnavailable(format!("Service call failed: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_circuit_breaker_trip() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            reset_timeout_ms: 100,
            half_open_limit: 1,
        };
        
        let cb = CircuitBreaker::new(config);
        
        // Circuit should start closed
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        // Record failures
        cb.on_failure().await;
        cb.on_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        // This should trip the circuit
        cb.on_failure().await;
        assert_eq!(cb.get_state().await, CircuitState::Open);
        
        // Request should be rejected
        assert!(cb.allow_request().await.is_err());
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Now should be half-open
        assert!(cb.allow_request().await.is_ok());
        assert_eq!(cb.get_state().await, CircuitState::HalfOpen);
        
        // Only one request allowed in half-open
        assert!(cb.allow_request().await.is_err());
        
        // Successful request should close the circuit
        cb.on_success().await;
        assert_eq!(cb.get_state().await, CircuitState::Closed);
        
        // Should allow requests again
        assert!(cb.allow_request().await.is_ok());
    }
    
    #[tokio::test]
    async fn test_circuit_protected() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            reset_timeout_ms: 100,
            half_open_limit: 1,
        };
        
        let cb = Arc::new(CircuitBreaker::new(config));
        let service = CircuitProtected::new(42, cb);
        
        // Successful call
        let result = service.execute(|val| async move { Ok::<_, &str>(*val) }).await;
        assert_eq!(result, Ok(42));
        
        // Failing call
        let result = service.execute(|_| async { Err::<u32, _>("error") }).await;
        assert!(result.is_err());
        
        // Another failing call should trip the circuit
        let result = service.execute(|_| async { Err::<u32, _>("error") }).await;
        assert!(result.is_err());
        
        // Circuit should be open now
        let result = service.execute(|val| async move { Ok::<_, &str>(*val) }).await;
        assert!(result.is_err());
        
        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(150)).await;
        
        // Should be half-open now
        let result = service.execute(|val| async move { Ok::<_, &str>(*val) }).await;
        assert_eq!(result, Ok(42));
        
        // Circuit should be closed again
        let result = service.execute(|val| async move { Ok::<_, &str>(*val) }).await;
        assert_eq!(result, Ok(42));
    }
}
