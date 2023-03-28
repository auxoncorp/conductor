use crate::stringy_newtype;
use derive_more::{AsRef, Deref, Display, From, Into};
use std::{collections::BTreeMap, path::PathBuf};

stringy_newtype!(SystemName);

impl Default for SystemName {
    fn default() -> Self {
        SystemName(conductor_config::DEFAULT_SYSTEM_NAME.to_owned())
    }
}

stringy_newtype!(ComponentName);
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

// TODO - clearly the display usage here isn't as it was intended, fixme
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}")]
pub enum ProviderKind {
    #[display(fmt = "{}", "self.as_str()")]
    Gazebo,
    #[display(fmt = "{}", "self.as_str()")]
    Renode,
    #[display(fmt = "{}", "self.as_str()")]
    Qemu,
    #[display(fmt = "{}", "self.as_str()")]
    Docker,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        use ProviderKind::*;
        match self {
            Gazebo => "gazebo",
            Renode => "renode",
            Qemu => "qemu",
            Docker => "docker",
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Display)]
#[display(fmt = "{}")]
pub enum ConnectionKind {
    #[display(fmt = "{}", "self.as_str()")]
    Uart,
    #[display(fmt = "{}", "self.as_str()")]
    Gpio,
    #[display(fmt = "{}", "self.as_str()")]
    Network,
}

impl ConnectionKind {
    /// Any connector to this connection can be either the
    /// initiator or the recipient of a transfer
    pub fn is_symmetrical(self) -> bool {
        use ConnectionKind::*;
        match self {
            Uart => true,
            Gpio => false,
            Network => true,
        }
    }

    pub fn as_str(self) -> &'static str {
        use ConnectionKind::*;
        match self {
            Uart => "uart",
            Gpio => "gpio",
            Network => "network",
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, AsRef, Deref, From, Into)]
pub struct EnvironmentVariableKeyValuePairs(pub(crate) BTreeMap<String, String>);

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, AsRef, Deref, From, Into)]
pub struct HostToGuestAssetPaths(pub(crate) BTreeMap<PathBuf, PathBuf>);

impl From<MachineName> for ComponentName {
    fn from(value: MachineName) -> Self {
        ComponentName(value.0)
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
