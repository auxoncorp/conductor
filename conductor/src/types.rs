use crate::name_newtype;
use derive_more::{AsRef, Deref, Display, From, Into};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

name_newtype!(SystemName);

impl Default for SystemName {
    fn default() -> Self {
        SystemName(conductor_config::DEFAULT_SYSTEM_NAME.to_owned())
    }
}

name_newtype!(ComponentName);
name_newtype!(WorldName);
name_newtype!(MachineName);
name_newtype!(ConnectionName);
name_newtype!(InterfaceName);

// TODO - not sure what the constraints for these need to be yet
pub type TapDevice = String;
pub type BridgeName = InterfaceName;

impl InterfaceName {
    const DEFAULT_IFACE_PREFIX: &'static str = "conductor";

    // TODO - this is more or less arbitrary, except it needs
    // to adhere to docker network API constraints, which need to be determined
    pub(crate) fn new_system_wired_network(iface_index: usize) -> Self {
        Self(format!("{}{}", Self::DEFAULT_IFACE_PREFIX, iface_index))
    }
}

// TODO - this will probably need to change
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, AsRef, Deref, Display, Into)]
pub struct ContainerRuntimeName(String);

impl ContainerRuntimeName {
    const DELIMITER: &'static str = "___";

    pub fn extract_components<S: AsRef<str>>(s: S) -> Option<(SystemName, Vec<ComponentName>)> {
        let parts: Vec<&str> = s.as_ref().split(Self::DELIMITER).collect();
        if parts.len() < 2 {
            None
        } else {
            let sys_name = SystemName::new_canonicalize(parts[0])?;
            let mut comps = Vec::new();
            for comp_name in &parts[1..] {
                let c = ComponentName::new_canonicalize(comp_name)?;
                comps.push(c);
            }
            Some((sys_name, comps))
        }
    }

    pub(crate) fn new_single(system: &SystemName, component: &ComponentName) -> Self {
        Self(format!("{system}{}{component}", Self::DELIMITER))
    }

    pub(crate) fn new_multi(system: &SystemName, components: &BTreeSet<ComponentName>) -> Self {
        debug_assert!(!components.is_empty());
        let mut name = format!("{system}");
        for comp in components.iter() {
            name.push_str(&format!("{}{comp}", Self::DELIMITER));
        }
        Self(name)
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
    Container,
}

impl ProviderKind {
    pub fn as_str(self) -> &'static str {
        use ProviderKind::*;
        match self {
            Gazebo => "gazebo",
            Renode => "renode",
            Qemu => "qemu",
            Container => "container",
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

    /// This connection kind requires all connectors to be hosted within
    /// the same container
    pub fn is_restricted_to_common_conatainer(&self) -> bool {
        use ConnectionKind::*;
        match self {
            Uart => false,
            Gpio => true,
            Network => false,
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

#[derive(Debug, thiserror::Error)]
#[error("Failed to merge environment variable '{_0}={_1}', already set to '{_2}'")]
pub struct EnvironmentVariableMergeConflict(pub String, pub String, pub String);

impl EnvironmentVariableKeyValuePairs {
    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), EnvironmentVariableMergeConflict> {
        for (k, v) in other.0.iter() {
            self.insert(k.clone(), v.clone())?;
        }
        Ok(())
    }

    // TODO pick a distinct error variant for this
    pub(crate) fn insert(
        &mut self,
        k: String,
        v: String,
    ) -> Result<(), EnvironmentVariableMergeConflict> {
        match self.0.insert(k.clone(), v.clone()) {
            None => Ok(()),
            Some(prev_val) => {
                if prev_val != v {
                    Err(EnvironmentVariableMergeConflict(k, v, prev_val))
                } else {
                    Ok(())
                }
            }
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default, AsRef, Deref, From, Into)]
pub struct HostToGuestAssetPaths(pub(crate) BTreeMap<PathBuf, PathBuf>);

#[derive(Debug, thiserror::Error)]
#[error("Failed to merge host-to-guest asset '{_0:?} -> {_1:?}', already set to '{_2:?}'")]
pub struct HostToGuestAssetPathMergeConflict(pub PathBuf, pub PathBuf, pub PathBuf);

// TODO - use these methods instead of manual conflict checks in config and deployment-plan
impl HostToGuestAssetPaths {
    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), HostToGuestAssetPathMergeConflict> {
        for (k, v) in other.0.iter() {
            self.insert(k.clone(), v.clone())?;
        }
        Ok(())
    }

    // TODO pick a distinct error variant for this
    pub(crate) fn insert(
        &mut self,
        k: PathBuf,
        v: PathBuf,
    ) -> Result<(), HostToGuestAssetPathMergeConflict> {
        match self.0.insert(k.clone(), v.clone()) {
            None => Ok(()),
            Some(prev_val) => {
                if prev_val != v {
                    Err(HostToGuestAssetPathMergeConflict(k, v, prev_val))
                } else {
                    Ok(())
                }
            }
        }
    }
}

impl From<WorldName> for ComponentName {
    fn from(value: WorldName) -> Self {
        ComponentName(value.0)
    }
}

impl From<MachineName> for ComponentName {
    fn from(value: MachineName) -> Self {
        ComponentName(value.0)
    }
}

#[macro_export]
macro_rules! name_newtype {
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
            // TODO - this constraint is a bit excessive, we should figure out what exactly
            // we can tolerate at which stages in the pipeline
            // this is only to satisfy the renode-script-specific use case, we can probably
            // get away with just doing a local-to-renode-script repr of these
            pub fn new_canonicalize<T: AsRef<str>>(s: T) -> Option<Self> {
                let inner: String = s
                    .as_ref()
                    .chars()
                    .map(|c| if c.is_whitespace() { '_' as _ } else { c })
                    .collect();
                if inner.is_empty() {
                    None
                } else {
                    Some(Self(inner))
                }
            }
        }

        impl std::str::FromStr for $t {
            type Err = String;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                Self::new_canonicalize(s)
                    .ok_or_else(|| format!("Cannot construct a {} from '{s}'", stringify!($t)))
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_names() {
        assert_eq!(
            MachineName::new_canonicalize("my machine"),
            Some(MachineName("my_machine".to_owned()))
        );
        assert_eq!(
            ComponentName::new_canonicalize("foo\t  \nbar"),
            Some(ComponentName("foo____bar".to_owned()))
        );
    }
}
