use std::{collections::HashMap, fmt};

use shell_core::NotchConfig;

use super::{ModuleContext, ModuleId, ModuleMetadata, ModuleSize, ModuleView, NotchModule};

/// Registration and activation errors are kept explicit so composition fails
/// before the GPUI runtime starts when a module is accidentally duplicated.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModuleRegistryError {
    Duplicate(ModuleId),
    Unknown(ModuleId),
}

impl fmt::Display for ModuleRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Duplicate(id) => write!(formatter, "module '{id}' is already registered"),
            Self::Unknown(id) => write!(formatter, "module '{id}' is not registered"),
        }
    }
}

impl std::error::Error for ModuleRegistryError {}

/// Host-owned registry for statically linked module implementations.
pub struct ModuleRegistry {
    modules: HashMap<ModuleId, Box<dyn NotchModule>>,
    order: Vec<ModuleId>,
    active: Option<ModuleId>,
}

impl Default for ModuleRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModuleRegistry {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            order: Vec::new(),
            active: None,
        }
    }

    pub fn register<M>(&mut self, module: M) -> Result<(), ModuleRegistryError>
    where
        M: NotchModule,
    {
        self.register_boxed(Box::new(module))
    }

    pub fn register_boxed(
        &mut self,
        module: Box<dyn NotchModule>,
    ) -> Result<(), ModuleRegistryError> {
        let id = module.metadata().id;
        if self.modules.contains_key(&id) {
            return Err(ModuleRegistryError::Duplicate(id));
        }
        self.order.push(id);
        self.modules.insert(id, module);
        Ok(())
    }

    pub fn lookup(&self, id: ModuleId) -> Option<&dyn NotchModule> {
        self.modules.get(&id).map(Box::as_ref)
    }

    pub fn lookup_mut(&mut self, id: ModuleId) -> Option<&mut dyn NotchModule> {
        self.modules.get_mut(&id).map(Box::as_mut)
    }

    pub fn activate(&mut self, id: ModuleId) -> Result<(), ModuleRegistryError> {
        if !self.modules.contains_key(&id) {
            return Err(ModuleRegistryError::Unknown(id));
        }
        self.active = Some(id);
        Ok(())
    }

    pub fn deactivate(&mut self) {
        self.active = None;
    }

    pub fn active(&self) -> Option<ModuleId> {
        self.active
    }

    pub fn metadata(&self) -> Vec<ModuleMetadata> {
        self.order
            .iter()
            .filter_map(|id| self.modules.get(id).map(|module| module.metadata()))
            .collect()
    }

    pub fn resolve_key(&self, key: &str) -> Option<ModuleId> {
        self.order.iter().copied().find(|id| {
            let metadata = self
                .modules
                .get(id)
                .expect("module order is consistent")
                .metadata();
            metadata.id.as_str() == key || metadata.aliases.contains(&key)
        })
    }

    pub fn preferred_size(&self, id: ModuleId, config: &NotchConfig) -> Option<ModuleSize> {
        self.lookup(id).map(|module| module.preferred_size(config))
    }

    pub fn render_key(&mut self, key: &str, mut context: ModuleContext) -> Option<ModuleView> {
        let id = self.resolve_key(key)?;
        context.module = id;
        context.module_key = key.to_owned();
        self.lookup_mut(id).map(|module| module.render(context))
    }

    pub fn render_module(
        &mut self,
        id: ModuleId,
        mut context: ModuleContext,
    ) -> Option<ModuleView> {
        context.module = id;
        context.module_key = id.as_str().to_owned();
        self.lookup_mut(id).map(|module| module.render(context))
    }

    pub fn render_configured(
        &mut self,
        keys: &[String],
        context: ModuleContext,
    ) -> Vec<ModuleView> {
        keys.iter()
            .filter_map(|key| self.render_key(key, context.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use gpui::{IntoElement, div};
    use shell_core::NotchConfig;

    use super::*;

    struct TestModule;

    impl NotchModule for TestModule {
        fn metadata(&self) -> ModuleMetadata {
            ModuleMetadata {
                id: ModuleId::Clock,
                title: "Clock",
                aliases: &[],
            }
        }

        fn preferred_size(&self, config: &NotchConfig) -> ModuleSize {
            ModuleSize::from_config(config, false)
        }

        fn render(&mut self, _context: ModuleContext) -> ModuleView {
            div().into_any_element()
        }
    }

    #[test]
    fn registration_activation_and_lookup_are_host_owned() {
        let mut registry = ModuleRegistry::new();
        registry.register(TestModule).expect("first registration");
        assert!(registry.lookup(ModuleId::Clock).is_some());
        registry.activate(ModuleId::Clock).expect("known module");
        assert_eq!(registry.active(), Some(ModuleId::Clock));
        assert_eq!(registry.metadata()[0].title, "Clock");
    }

    #[test]
    fn duplicate_registration_is_rejected() {
        let mut registry = ModuleRegistry::new();
        registry.register(TestModule).expect("first registration");
        assert_eq!(
            registry.register(TestModule),
            Err(ModuleRegistryError::Duplicate(ModuleId::Clock))
        );
    }
}
