//! Internal Rust contract between the Notch host and statically linked modules.

mod context;
mod module;
mod registry;

pub use context::ModuleContext;
pub use module::{ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule, NotchRoute};
pub use registry::{ModuleRegistry, ModuleRegistryError};
