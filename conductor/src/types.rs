use crate::stringy_newtype;
use derive_more::{AsRef, Deref, Display, Into};

stringy_newtype!(SystemName);

impl Default for SystemName {
    fn default() -> Self {
        SystemName(conductor_config::DEFAULT_SYSTEM_NAME.to_owned())
    }
}

stringy_newtype!(WorldName);
stringy_newtype!(MachineName);
stringy_newtype!(ConnectionName);
stringy_newtype!(InterfaceName);

// TODO(jon@auxon.io) just a place holder/example
// we still need a canonical repr and type
// possibly runtime (container, etc) defined
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, AsRef, Deref, Display, Into)]
pub struct MachineRuntimeId(String);

impl MachineRuntimeId {
    pub fn new(system: &SystemName, machine: &MachineName) -> Self {
        Self(format!("{system}::{machine}"))
    }
}

#[macro_export]
macro_rules! stringy_newtype {
    ($t:ident) => {
        #[derive(
            Clone,
            Eq,
            PartialEq,
            Ord,
            PartialOrd,
            Hash,
            Debug,
            derive_more::AsRef,
            derive_more::Deref,
            derive_more::Display,
            derive_more::Into,
        )]
        pub struct $t(String);

        impl AsRef<str> for $t {
            fn as_ref(&self) -> &str {
                &self.0
            }
        }

        impl $t {
            pub fn new<T: AsRef<str>>(s: T) -> Option<Self> {
                let s: &str = s.as_ref();
                if s.is_empty() {
                    None
                } else {
                    Some(Self(s.to_owned()))
                }
            }
        }
    };
}
