use crate::{
    component::Component,
    config::Connection,
    types::{ComponentName, ConnectionName},
};
use itertools::Itertools;
use petgraph::{dot::Dot, graph::UnGraph};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt, io,
};

// TODO
// methods for
// getting a Map<ContainerThing, List<Component>>
//   usualy 1-to-1 like qemu
//   can be 1-to-n for multi-machines on renode
//   based on the edges (is-renode && is-special-renode-local connection -> same container)
pub struct ComponentGraph<N> {
    g: InnerGraph<N>,
}

type InnerGraph<N> = UnGraph<N, Connection>;

impl<N: fmt::Display> ComponentGraph<N> {
    pub fn new<C, E>(components: C, connections: E) -> Self
    where
        N: Component + Clone,
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
            for connector in comp.connectors().iter() {
                let entry = connections_to_components
                    .entry(connector.name.clone())
                    .or_default();
                entry.insert(comp.name());
            }
        }

        for (conn_name, comp_names) in connections_to_components.into_iter() {
            if comp_names.len() > 1 {
                for comp_pair in comp_names.into_iter().permutations(2) {
                    let a = component_to_node_idx.get(&comp_pair[0]).unwrap();
                    let b = component_to_node_idx.get(&comp_pair[1]).unwrap();
                    let e = connections.get(&conn_name).unwrap();
                    g.update_edge(*a, *b, e.clone());
                }
            }
        }

        Self { g }
    }

    /// Return set of components within a parent container based on the connection
    /// constraints imposed by the components provider
    pub fn components_by_container(&self) -> BTreeSet<BTreeSet<N>> {
        todo!()
    }

    pub fn write_dot<W: io::Write>(&self, out: &mut W) -> io::Result<()> {
        let dot = Dot::with_config(&self.g, &[]);
        writeln!(out, "{dot}")?;
        Ok(())
    }
}
