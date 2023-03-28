use crate::{
    component::Component,
    config::Connection,
    types::{ComponentName, ConnectionName},
};
use colorous::{Color, CATEGORY10};
use itertools::Itertools;
use petgraph::{
    dot::Dot,
    graph::{EdgeReference, NodeIndex, UnGraph},
    visit::EdgeRef,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, io,
};

// TODO
// synthesize host node when involved
pub struct ComponentGraph<N> {
    connections: BTreeMap<ConnectionName, Connection>,
    components: BTreeMap<ComponentName, N>,

    /// Map of connections and the set of components that are connected to that connection
    connections_to_components: BTreeMap<ConnectionName, BTreeSet<ComponentName>>,

    /// Set of components within a parent container based on the connection
    /// constraints imposed by the component's provider
    components_by_container: BTreeSet<BTreeSet<ComponentName>>,

    g: InnerGraph<N>,
}

type InnerGraph<N> = UnGraph<N, Connection>;

impl<N: Component + fmt::Display + Clone> ComponentGraph<N> {
    pub fn new<C, E>(components: C, connections: E) -> Self
    where
        C: IntoIterator<Item = N>,
        E: IntoIterator<Item = Connection>,
    {
        let mut g = InnerGraph::default();

        let connections: BTreeMap<ConnectionName, Connection> = connections
            .into_iter()
            .map(|c| (c.name().clone(), c))
            .collect();

        let components: BTreeMap<ComponentName, N> =
            components.into_iter().map(|c| (c.name(), c)).collect();

        let mut component_to_node_idx = BTreeMap::new();
        for node in components.values() {
            let name = node.name().clone();
            let node_idx = g.add_node(node.clone());
            component_to_node_idx.insert(name, node_idx);
        }

        // What components are connected to this connection
        let mut connections_to_components: BTreeMap<ConnectionName, BTreeSet<ComponentName>> =
            BTreeMap::new();
        for comp in components.values() {
            let connectors = comp.connectors();
            for connector in connectors.iter() {
                let entry = connections_to_components
                    .entry(connector.name().clone())
                    .or_default();
                entry.insert(comp.name());
            }
        }

        for (conn_name, comp_names) in connections_to_components.iter() {
            // TODO - host-bridge will be a thing soon
            if comp_names.len() > 1 {
                for comp_pair in comp_names.iter().permutations(2) {
                    let a = component_to_node_idx.get(comp_pair[0]).unwrap();
                    let b = component_to_node_idx.get(comp_pair[1]).unwrap();
                    let e = connections.get(conn_name).unwrap();
                    g.update_edge(*a, *b, e.clone());
                }
            }
        }

        // NOTE: this isn't very effecient, it dups/throws-away a lot
        // we could instead actually walk the graph instead of
        // just immediate neighboring components
        let mut containers_with_dups = BTreeSet::new();
        for node_idx in g.node_indices() {
            let comp_name = g.node_weight(node_idx).map(|c| c.name()).unwrap();

            // Always start with 1-to-1 container-to-component relationship
            let mut container = Container::from(comp_name);

            // Group components with neighboring connections that require a single container
            for edge in g.edges(node_idx) {
                let connection = edge.weight();

                if connection.kind().is_restricted_to_common_conatainer() {
                    let other_comp = g.node_weight(edge.target()).map(|c| c.name()).unwrap();
                    container.0.insert(other_comp);
                }
            }
            containers_with_dups.insert(container.0);
        }
        // Merge containers with common components
        let mut components_by_container = BTreeSet::new();
        for comp_name in components.keys() {
            let mut cont = Container::default();
            for c in containers_with_dups.iter() {
                if c.contains(comp_name) {
                    cont.merge(c.clone());
                }
            }
            components_by_container.insert(cont.0);
        }

        Self {
            connections,
            components,
            connections_to_components,
            components_by_container,
            g,
        }
    }

    pub fn connections(&self) -> &BTreeMap<ConnectionName, Connection> {
        &self.connections
    }

    pub fn components(&self) -> &BTreeMap<ComponentName, N> {
        &self.components
    }

    pub fn connections_to_components(&self) -> &BTreeMap<ConnectionName, BTreeSet<ComponentName>> {
        &self.connections_to_components
    }

    pub fn components_by_container(&self) -> &BTreeSet<BTreeSet<ComponentName>> {
        &self.components_by_container
    }

    pub fn write_dot<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        let connection_colors: BTreeMap<ConnectionName, Color> = self
            .connections_to_components
            .keys()
            .cloned()
            .enumerate()
            .map(|(idx, name)| (name, CATEGORY10[idx % CATEGORY10.len()]))
            .collect();

        let component_colors: BTreeMap<ComponentName, Color> = self
            .components_by_container
            .iter()
            .enumerate()
            .flat_map(|(idx, comps)| {
                comps
                    .iter()
                    .cloned()
                    .map(move |name| (name, CATEGORY10[idx % CATEGORY10.len()]))
            })
            .collect();

        let get_node_attributes = |_g: &InnerGraph<N>, node_ref: (NodeIndex, &N)| {
            let name = node_ref.1.name();
            let node_color = component_colors.get(&name).unwrap();
            format!("penwidth = 2.0 color=\"#{node_color:X}\"")
        };

        let get_edge_attributes = |g: &InnerGraph<N>, edge_ref: EdgeReference<'_, Connection>| {
            let connection = edge_ref.weight();
            let edge_color = connection_colors.get(connection.name()).unwrap();
            let dir = if connection.kind().is_symmetrical() {
                "both"
            } else {
                let src_comp = &g[edge_ref.source()];

                let dir = src_comp
                    .connectors()
                    .iter()
                    .find_map(|c| {
                        if c.name() == connection.name() {
                            Some(match c.is_asymmetrical_initiator() {
                                Some(true) => "forward",
                                Some(false) => "back",
                                None => "both",
                            })
                        } else {
                            None
                        }
                    })
                    .unwrap_or("both");

                dir
            };

            format!("dir={dir} color=\"#{edge_color:X}\"")
        };

        let dot = Dot::with_attr_getters(&self.g, &[], &get_edge_attributes, &get_node_attributes);
        writeln!(out, "{dot}")?;
        Ok(())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Default)]
struct Container(BTreeSet<ComponentName>);

impl Container {
    fn from(comp: ComponentName) -> Self {
        let mut c = Self::default();
        c.0.insert(comp);
        c
    }

    fn merge(&mut self, other: BTreeSet<ComponentName>) {
        for comp in other.into_iter() {
            self.0.insert(comp);
        }
    }
}
