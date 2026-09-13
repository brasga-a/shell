//! Linux service adapter boundary.
//!
//! Concrete D-Bus, audio, power, network, and media integrations belong here
//! as they acquire an owned use case. The application does not depend on
//! those native clients directly.

use shell_core::PlatformError;

#[derive(Debug, Default)]
pub struct LinuxServices;

impl LinuxServices {
    pub fn initialize() -> Result<Self, PlatformError> {
        Ok(Self)
    }
}
