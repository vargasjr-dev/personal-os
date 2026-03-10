pub mod anthropic;
pub mod local;

use core::fmt;

/// Result type for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

/// Errors that can occur during LLM operations
#[derive(Debug, Clone, Copy)]
pub enum LlmError {
    /// Network error (API unreachable)
    NetworkError,
    /// API error (authentication, rate limit, etc.)
    ApiError,
    /// Local model not loaded
    ModelNotLoaded,
    /// Invalid input
    InvalidInput,
    /// Timeout
    Timeout,
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LlmError::NetworkError => write!(f, "Network error"),
            LlmError::ApiError => write!(f, "API error"),
            LlmError::ModelNotLoaded => write!(f, "Local model not loaded"),
            LlmError::InvalidInput => write!(f, "Invalid input"),
            LlmError::Timeout => write!(f, "Request timeout"),
        }
    }
}

/// Response from an LLM query
#[derive(Debug)]
pub struct LlmResponse {
    pub text: &'static str,
    pub tokens_used: usize,
    pub latency_ms: u64,
}

/// Core trait for LLM backends
/// 
/// Implement this trait to add new LLM providers (Anthropic, local Llama, etc.)
/// The kernel can swap backends at runtime without changing calling code.
pub trait LlmBackend {
    /// Query the LLM with a prompt
    fn query(&self, prompt: &str) -> LlmResult<LlmResponse>;
    
    /// Get the backend name
    fn name(&self) -> &'static str;
    
    /// Check if the backend is available/ready
    fn is_ready(&self) -> bool;
    
    /// Initialize the backend (load model, authenticate API, etc.)
    fn initialize(&mut self) -> LlmResult<()>;
}

/// Global LLM manager that holds the active backend
/// 
/// This allows the OS to switch between cloud (Anthropic) and local (Llama)
/// based on availability, performance needs, or user preference.
pub struct LlmManager {
    backend: BackendType,
}

pub enum BackendType {
    Anthropic(anthropic::AnthropicBackend),
    Local(local::LocalLlamaBackend),
}

impl LlmManager {
    /// Create a new LLM manager with the specified backend
    pub fn new(use_local: bool) -> Self {
        let backend = if use_local {
            BackendType::Local(local::LocalLlamaBackend::new())
        } else {
            BackendType::Anthropic(anthropic::AnthropicBackend::new())
        };
        
        LlmManager { backend }
    }
    
    /// Query the active backend
    pub fn query(&self, prompt: &str) -> LlmResult<LlmResponse> {
        match &self.backend {
            BackendType::Anthropic(backend) => backend.query(prompt),
            BackendType::Local(backend) => backend.query(prompt),
        }
    }
    
    /// Get the name of the active backend
    pub fn backend_name(&self) -> &'static str {
        match &self.backend {
            BackendType::Anthropic(backend) => backend.name(),
            BackendType::Local(backend) => backend.name(),
        }
    }
    
    /// Check if the active backend is ready
    pub fn is_ready(&self) -> bool {
        match &self.backend {
            BackendType::Anthropic(backend) => backend.is_ready(),
            BackendType::Local(backend) => backend.is_ready(),
        }
    }
    
    /// Switch to a different backend
    pub fn switch_backend(&mut self, use_local: bool) {
        self.backend = if use_local {
            BackendType::Local(local::LocalLlamaBackend::new())
        } else {
            BackendType::Anthropic(anthropic::AnthropicBackend::new())
        };
    }
}
