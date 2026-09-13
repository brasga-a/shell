macro_rules! opaque_id {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        #[repr(transparent)]
        pub struct $name($inner);

        impl $name {
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self::new(value)
            }
        }
    };
}

opaque_id!(OutputId, u64);
opaque_id!(WorkspaceId, i64);
opaque_id!(WindowId, u64);
opaque_id!(SurfaceId, u64);

impl std::fmt::Display for OutputId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(formatter)
    }
}

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(formatter)
    }
}

impl std::fmt::Display for WindowId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(formatter)
    }
}

impl std::fmt::Display for SurfaceId {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.get().fmt(formatter)
    }
}

#[cfg(test)]
mod tests {
    use super::{OutputId, WorkspaceId};

    #[test]
    fn ids_are_opaque_and_value_based() {
        let output = OutputId::new(7);
        let workspace = WorkspaceId::new(-1);

        assert_eq!(output.get(), 7);
        assert_eq!(workspace.get(), -1);
        assert_eq!(output, OutputId::from(7));
    }
}
