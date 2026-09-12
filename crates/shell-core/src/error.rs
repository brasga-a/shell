use std::fmt::{Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompositorError {
    Unavailable { message: String },
    Connection { message: String },
    Operation { operation: String, message: String },
    InvalidData { message: String },
    Unsupported { operation: String },
}

impl CompositorError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable {
            message: message.into(),
        }
    }

    pub fn unsupported(operation: impl Into<String>) -> Self {
        Self::Unsupported {
            operation: operation.into(),
        }
    }
}

impl Display for CompositorError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { message } => write!(formatter, "compositor unavailable: {message}"),
            Self::Connection { message } => {
                write!(formatter, "compositor connection failed: {message}")
            }
            Self::Operation { operation, message } => {
                write!(
                    formatter,
                    "compositor operation '{operation}' failed: {message}"
                )
            }
            Self::InvalidData { message } => {
                write!(formatter, "invalid compositor data: {message}")
            }
            Self::Unsupported { operation } => {
                write!(formatter, "unsupported compositor operation: {operation}")
            }
        }
    }
}

impl std::error::Error for CompositorError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    Unavailable { message: String },
    Read { message: String },
    Write { message: String },
    Parse { message: String },
    Validation { message: String },
    UnsupportedVersion { found: u32, supported: u32 },
}

impl ConfigError {
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable {
            message: message.into(),
        }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    pub fn version(found: u32, supported: u32) -> Self {
        Self::UnsupportedVersion { found, supported }
    }
}

impl Display for ConfigError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { message } => {
                write!(formatter, "configuration unavailable: {message}")
            }
            Self::Read { message } => write!(formatter, "configuration read failed: {message}"),
            Self::Write { message } => write!(formatter, "configuration write failed: {message}"),
            Self::Parse { message } => write!(formatter, "configuration parse failed: {message}"),
            Self::Validation { message } => write!(formatter, "invalid configuration: {message}"),
            Self::UnsupportedVersion { found, supported } => write!(
                formatter,
                "configuration schema version {found} is newer than supported version {supported}"
            ),
        }
    }
}

impl std::error::Error for ConfigError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PlatformError {
    Unavailable { message: String },
    Initialization { message: String },
    Runtime { message: String },
    Shutdown { message: String },
    Unsupported { capability: String },
}

impl PlatformError {
    pub fn initialization(message: impl Into<String>) -> Self {
        Self::Initialization {
            message: message.into(),
        }
    }
}

impl Display for PlatformError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable { message } => write!(formatter, "platform unavailable: {message}"),
            Self::Initialization { message } => {
                write!(formatter, "platform initialization failed: {message}")
            }
            Self::Runtime { message } => write!(formatter, "platform runtime failure: {message}"),
            Self::Shutdown { message } => write!(formatter, "platform shutdown failed: {message}"),
            Self::Unsupported { capability } => {
                write!(formatter, "unsupported platform capability: {capability}")
            }
        }
    }
}

impl std::error::Error for PlatformError {}
