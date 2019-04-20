use crate::ich::Fingerprint;
use rustc_data_structures::fx::FxHashMap;
use rustc_data_structures::indexed_vec::IndexVec;
use super::dep_node::DepNode;
use super::graph::{DepNodeIndex, DepNodeState};
use super::serialized::SerializedDepGraph;

#[derive(Debug, /*RustcEncodable, RustcDecodable,*/ Default)]
pub struct PreviousDepGraph {
    /// Maps from dep nodes to their previous index, if any.
    index: FxHashMap<DepNode, DepNodeIndex>,
    /// The set of all DepNodes in the graph
    nodes: IndexVec<DepNodeIndex, DepNode>,
    /// The set of all Fingerprints in the graph. Each Fingerprint corresponds to
    /// the DepNode at the same index in the nodes vector.
    fingerprints: IndexVec<DepNodeIndex, Fingerprint>,
    /// For each DepNode, stores the list of edges originating from that
    /// DepNode. Encoded as a [start, end) pair indexing into edge_list_data,
    /// which holds the actual DepNodeIndices of the target nodes.
    edge_list_indices: IndexVec<DepNodeIndex, (u32, u32)>,
    /// A flattened list of all edge targets in the graph. Edge sources are
    /// implicit in edge_list_indices.
    edge_list_data: Vec<DepNodeIndex>,
    /// A set of nodes which are no longer valid.
    pub(super) state: IndexVec<DepNodeIndex, DepNodeState>,
}

impl PreviousDepGraph {
    pub fn new(graph: SerializedDepGraph) -> PreviousDepGraph {
        let index: FxHashMap<_, _> = graph.nodes
            .iter_enumerated()
            .map(|(idx, dep_node)| (dep_node.node, idx))
            .collect();

        let fingerprints: IndexVec<DepNodeIndex, _> =
            graph.nodes.iter().map(|d| d.fingerprint).collect();
        let nodes: IndexVec<DepNodeIndex, _> =
            graph.nodes.iter().map(|d| d.node).collect();

        let total_edge_count: usize = graph.nodes.iter().map(|d| d.edges.len()).sum();

        let mut edge_list_indices = IndexVec::with_capacity(nodes.len());
        let mut edge_list_data = Vec::with_capacity(total_edge_count);

        for (current_dep_node_index, edges) in graph.nodes.iter_enumerated()
                                                                .map(|(i, d)| (i, &d.edges)) {
            let start = edge_list_data.len() as u32;
            edge_list_data.extend(edges.iter().cloned());
            let end = edge_list_data.len() as u32;

            debug_assert_eq!(current_dep_node_index.index(), edge_list_indices.len());
            edge_list_indices.push((start, end));
        }

        debug_assert!(edge_list_data.len() <= ::std::u32::MAX as usize);
        debug_assert_eq!(edge_list_data.len(), total_edge_count);

        PreviousDepGraph {
            fingerprints,
            nodes,
            edge_list_indices,
            edge_list_data,
            index,
            state: graph.state.convert_index_type(),
        }
    }

    #[inline]
    pub fn edge_targets_from(
        &self,
        dep_node_index: DepNodeIndex
    ) -> &[DepNodeIndex] {
        let targets = self.edge_list_indices[dep_node_index];
        &self.edge_list_data[targets.0 as usize..targets.1 as usize]
    }

    #[inline]
    pub fn index_to_node(&self, dep_node_index: DepNodeIndex) -> DepNode {
        self.nodes[dep_node_index]
    }

    #[inline]
    pub fn node_to_index(&self, dep_node: &DepNode) -> DepNodeIndex {
        self.index[dep_node]
    }

    #[inline]
    pub fn node_to_index_opt(&self, dep_node: &DepNode) -> Option<DepNodeIndex> {
        self.index.get(dep_node).cloned()
    }

    #[inline]
    pub fn fingerprint_of(&self, dep_node: &DepNode) -> Option<Fingerprint> {
        self.index
            .get(dep_node)
            .map(|&node_index| self.fingerprints[node_index])
    }

    #[inline]
    pub fn fingerprint_by_index(&self,
                                dep_node_index: DepNodeIndex)
                                -> Fingerprint {
        self.fingerprints[dep_node_index]
    }

    pub fn node_count(&self) -> usize {
        self.index.len()
    }
}
