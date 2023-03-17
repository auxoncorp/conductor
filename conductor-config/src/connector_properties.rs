use crate::MachineConnector;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConnectorPropertiesError {
    #[error("Failed to parse connector '{name}' UART properties")]
    ParseUart {
        name: String,
        #[source]
        error: Box<toml::de::Error>,
    },
    #[error("Failed to parse connector '{name}' GPIO properties")]
    ParseGpio {
        name: String,
        #[source]
        error: Box<toml::de::Error>,
    },
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct UartConnectorProperties {
    // TODO(jon@auxon.io) we may abstract some of this and move it to the
    // connection/definition level and generate this as backend-specific
    // props
    pub pipe: Option<PathBuf>,
    pub pty: Option<PathBuf>,
    pub port: Option<u16>,
    pub emit_config: Option<bool>,
}

impl TryFrom<&MachineConnector> for UartConnectorProperties {
    type Error = ConnectorPropertiesError;

    fn try_from(value: &MachineConnector) -> Result<Self, Self::Error> {
        let props =
            value
                .context
                .clone()
                .try_into()
                .map_err(|e| ConnectorPropertiesError::ParseUart {
                    name: value.name.clone(),
                    error: Box::new(e),
                })?;
        Ok(props)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct GpioConnectorProperties {
    pub source_pin: Option<u16>,
    pub destination_pin: Option<u16>,
}

impl TryFrom<&MachineConnector> for GpioConnectorProperties {
    type Error = ConnectorPropertiesError;

    fn try_from(value: &MachineConnector) -> Result<Self, Self::Error> {
        let props =
            value
                .context
                .clone()
                .try_into()
                .map_err(|e| ConnectorPropertiesError::ParseGpio {
                    name: value.name.clone(),
                    error: Box::new(e),
                })?;
        Ok(props)
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct NetworkConnectorProperties {
    // TODO(jon@auxon.io) TBD
}

impl TryFrom<&MachineConnector> for NetworkConnectorProperties {
    type Error = ConnectorPropertiesError;

    fn try_from(_value: &MachineConnector) -> Result<Self, Self::Error> {
        Ok(NetworkConnectorProperties {})
    }
}
