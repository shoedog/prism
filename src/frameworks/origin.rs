//! Data-origin classification (moved verbatim from `algorithms/provenance_slice.rs`, #702).

/// Classification of a data origin.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// User/client input (request body, query params, form data, stdin)
    UserInput,
    /// Configuration values (config files, settings)
    Config,
    /// Database query results
    Database,
    /// Hardcoded constant or literal
    Constant,
    /// Environment variable
    EnvVar,
    /// Function parameter (origin unknown without interprocedural analysis)
    FunctionParam,
    /// Return value from external/library function
    ExternalCall,
    /// Hardware / device I/O (ioctl, mmap, register reads) — C/C++ embedded
    Hardware,
    /// Could not determine origin
    Unknown,
}

impl Origin {
    pub fn risk_level(&self) -> &str {
        match self {
            Self::UserInput => "HIGH — requires validation/sanitization",
            Self::Hardware => "HIGH — raw device data, validate before use",
            Self::Database => "MEDIUM — may contain user-supplied data",
            Self::ExternalCall => "MEDIUM — verify return contract",
            Self::FunctionParam => "MEDIUM — depends on caller",
            Self::EnvVar => "LOW — trusted but may be misconfigured",
            Self::Config => "LOW — typically trusted",
            Self::Constant => "NONE — hardcoded value",
            Self::Unknown => "UNKNOWN — could not trace",
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Self::UserInput => "user_input",
            Self::Config => "config",
            Self::Database => "database",
            Self::Constant => "constant",
            Self::EnvVar => "env_var",
            Self::FunctionParam => "function_param",
            Self::ExternalCall => "external_call",
            Self::Hardware => "hardware",
            Self::Unknown => "unknown",
        }
    }
}
