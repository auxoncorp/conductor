use anyhow::{anyhow, bail, Context as _, Result};
use bollard::{network::CreateNetworkOptions, Docker};
use std::borrow::Cow;
use std::collections::HashMap;
use tracing::{debug, trace, warn};

#[derive(Debug, Default)]
pub struct NetworkBuilder {
    pub(crate) name: Option<String>,
}

impl NetworkBuilder {
    pub fn name<'a>(&mut self, name: impl Into<Cow<'a, str>>) -> &mut Self {
        self.name = Some(name.into().to_string());

        self
    }

    pub async fn resolve(&mut self) -> Result<Network> {
        let client = Docker::connect_with_local_defaults()
            .context("connect to container system service")?
            .negotiate_version()
            .await?;

        // take ownership of self's contents
        let mut src = Default::default();
        std::mem::swap(self, &mut src);

        let Some(name) = src.name else {
            bail!("name required");
        };

        let labels = HashMap::from([
            ("io.auxon.conductor", ""),
            ("io.auxon.conductor.system", "TODO"),
        ]);

        // TODO: Does it make sense to only create network if it doesn't already exist? The only
        // thing I have to go on here at the moment is name and that seems less than helpful.
        //
        // * Get system (?) metadata into here.
        // * Create networks with metadata
        // * Query networks by label metadata.
        //
        //let labels_filter = labels.iter().map(|(k, v)| format!("{k}={v}")).collect();
        //trace!("labels filter: {labels_filter:?}");

        //let list_networks_options = ListNetworksOptions {
        //    filters: HashMap::from([
        //        ("name".to_string(), vec![name.clone()]),
        //        ("label".to_string(), labels_filter),
        //    ]),
        //};
        //let networks = client.list_networks(Some(list_networks_options)).await?;
        //if let Some(network) = networks.get(0) {
        //    let state = NetworkState::Built {
        //        id: network.id.clone().unwrap(),
        //    };
        //    return Ok(Network { state, name });
        //}

        let labels_ref = labels.iter().map(|(k, v)| (*k, *v)).collect();

        trace!("labels: {labels_ref:?}");

        let network_options = CreateNetworkOptions {
            name: name.as_str(),
            labels: labels_ref,
            ..Default::default()
        };
        let create_network_resp = client.create_network(network_options).await?;

        if let Some(warning) = create_network_resp.warning {
            warn!("warning from container server while creating network: {warning}");
        }

        let network_id = create_network_resp.id.ok_or(anyhow!(
            "container service successfully created network, but did not return an id"
        ))?;

        debug!(network_id, name, "network created");

        Ok(Network {
            name,
            state: NetworkState::Built { id: network_id },
        })
    }
}

#[derive(Debug, Default, Clone)]
pub enum NetworkState {
    #[default]
    Defined,
    Built {
        id: String,
    },
}

#[derive(Debug, Clone)]
pub struct Network {
    pub(crate) state: NetworkState,
    pub(crate) name: String,
}

impl Network {
    pub fn builder() -> NetworkBuilder {
        Default::default()
    }
}
