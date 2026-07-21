use etas_core::id_type;

use super::GraphView;
use crate::Observer;

id_type!(NodeId);
id_type!(EdgeId);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node<N> {
    pub id: NodeId,
    pub value: N,
    first_out: Option<EdgeId>,
    first_in: Option<EdgeId>,
}

impl<N> Node<N> {
    pub fn first_out(&self) -> Option<EdgeId> {
        self.first_out
    }

    pub fn first_in(&self) -> Option<EdgeId> {
        self.first_in
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edge<E> {
    pub id: EdgeId,
    pub source: NodeId,
    pub target: NodeId,
    pub value: E,
    next_out: Option<EdgeId>,
    next_in: Option<EdgeId>,
}

impl<E> Edge<E> {
    pub fn next_out(&self) -> Option<EdgeId> {
        self.next_out
    }

    pub fn next_in(&self) -> Option<EdgeId> {
        self.next_in
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphEvent {
    NodeAdded {
        node: NodeId,
    },
    NodeRemoving {
        node: NodeId,
    },
    NodeRemoved {
        node: NodeId,
    },
    EdgeAdded {
        edge: EdgeId,
        source: NodeId,
        target: NodeId,
    },
    EdgeRemoving {
        edge: EdgeId,
        source: NodeId,
        target: NodeId,
    },
    EdgeRemoved {
        edge: EdgeId,
        source: NodeId,
        target: NodeId,
    },
}

pub struct Graph<N, E> {
    nodes: Vec<Option<Node<N>>>,
    edges: Vec<Option<Edge<E>>>,
    live_nodes: usize,
    live_edges: usize,
    observers: Vec<Box<dyn Observer<Graph<N, E>, GraphEvent>>>,
}

impl<N, E> Default for Graph<N, E> {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            live_nodes: 0,
            live_edges: 0,
            observers: Vec::new(),
        }
    }
}

impl<N, E> Graph<N, E> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, observer: impl Observer<Graph<N, E>, GraphEvent> + 'static) {
        self.observers.push(Box::new(observer));
    }

    pub fn add_node(&mut self, value: N) -> NodeId {
        let id = NodeId(self.nodes.len().min(u32::MAX as usize) as u32);
        self.nodes.push(Some(Node {
            id,
            value,
            first_out: None,
            first_in: None,
        }));
        self.live_nodes += 1;
        self.notify(GraphEvent::NodeAdded { node: id });
        id
    }

    pub fn add_edge(&mut self, source: NodeId, target: NodeId, value: E) -> Option<EdgeId> {
        if !self.contains_node(source) || !self.contains_node(target) {
            return None;
        }

        let id = EdgeId(self.edges.len().min(u32::MAX as usize) as u32);
        let next_out = self.nodes[source.index()].as_ref()?.first_out;
        let next_in = self.nodes[target.index()].as_ref()?.first_in;

        self.edges.push(Some(Edge {
            id,
            source,
            target,
            value,
            next_out,
            next_in,
        }));

        self.nodes[source.index()].as_mut()?.first_out = Some(id);
        self.nodes[target.index()].as_mut()?.first_in = Some(id);
        self.live_edges += 1;
        self.notify(GraphEvent::EdgeAdded {
            edge: id,
            source,
            target,
        });
        Some(id)
    }

    pub fn remove_edge(&mut self, edge: EdgeId) -> Option<Edge<E>> {
        let (source, target, next_out, next_in) = {
            let edge_ref = self.edge(edge)?;
            (
                edge_ref.source,
                edge_ref.target,
                edge_ref.next_out,
                edge_ref.next_in,
            )
        };
        self.notify(GraphEvent::EdgeRemoving {
            edge,
            source,
            target,
        });
        let removed = self.edges.get_mut(edge.index())?.take()?;
        self.unlink_out_edge(source, edge, next_out);
        self.unlink_in_edge(target, edge, next_in);
        self.live_edges -= 1;
        self.notify(GraphEvent::EdgeRemoved {
            edge,
            source,
            target,
        });
        Some(removed)
    }

    pub fn remove_node(&mut self, node: NodeId) -> Option<Node<N>> {
        if !self.contains_node(node) {
            return None;
        }
        self.notify(GraphEvent::NodeRemoving { node });

        let edge_ids = self
            .outgoing_edges(node)
            .into_iter()
            .chain(self.incoming_edges(node))
            .collect::<Vec<_>>();
        for edge in edge_ids {
            self.remove_edge(edge);
        }

        let removed = self.nodes.get_mut(node.index())?.take()?;
        self.live_nodes -= 1;
        self.notify(GraphEvent::NodeRemoved { node });
        Some(removed)
    }

    pub fn node(&self, node: NodeId) -> Option<&Node<N>> {
        self.nodes.get(node.index())?.as_ref()
    }

    pub fn node_mut(&mut self, node: NodeId) -> Option<&mut Node<N>> {
        self.nodes.get_mut(node.index())?.as_mut()
    }

    pub fn node_value(&self, node: NodeId) -> Option<&N> {
        self.node(node).map(|node| &node.value)
    }

    pub fn node_value_mut(&mut self, node: NodeId) -> Option<&mut N> {
        self.node_mut(node).map(|node| &mut node.value)
    }

    pub fn edge(&self, edge: EdgeId) -> Option<&Edge<E>> {
        self.edges.get(edge.index())?.as_ref()
    }

    pub fn edge_mut(&mut self, edge: EdgeId) -> Option<&mut Edge<E>> {
        self.edges.get_mut(edge.index())?.as_mut()
    }

    pub fn edge_value(&self, edge: EdgeId) -> Option<&E> {
        self.edge(edge).map(|edge| &edge.value)
    }

    pub fn edge_value_mut(&mut self, edge: EdgeId) -> Option<&mut E> {
        self.edge_mut(edge).map(|edge| &mut edge.value)
    }

    pub fn endpoints(&self, edge: EdgeId) -> Option<(NodeId, NodeId)> {
        let edge = self.edge(edge)?;
        Some((edge.source, edge.target))
    }

    pub fn nodes(&self) -> Vec<NodeId> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.as_ref().map(|_| NodeId(index as u32)))
            .collect()
    }

    pub fn edges(&self) -> Vec<EdgeId> {
        self.edges
            .iter()
            .enumerate()
            .filter_map(|(index, edge)| edge.as_ref().map(|_| EdgeId(index as u32)))
            .collect()
    }

    pub fn outgoing_edges(&self, node: NodeId) -> Vec<EdgeId> {
        let mut result = Vec::new();
        let mut current = self.node(node).and_then(Node::first_out);
        while let Some(edge) = current {
            if let Some(edge_ref) = self.edge(edge) {
                result.push(edge);
                current = edge_ref.next_out;
            } else {
                break;
            }
        }
        result
    }

    pub fn incoming_edges(&self, node: NodeId) -> Vec<EdgeId> {
        let mut result = Vec::new();
        let mut current = self.node(node).and_then(Node::first_in);
        while let Some(edge) = current {
            if let Some(edge_ref) = self.edge(edge) {
                result.push(edge);
                current = edge_ref.next_in;
            } else {
                break;
            }
        }
        result
    }

    pub fn successors(&self, node: NodeId) -> Vec<NodeId> {
        self.outgoing_edges(node)
            .into_iter()
            .filter_map(|edge| self.edge(edge).map(|edge| edge.target))
            .collect()
    }

    pub fn predecessors(&self, node: NodeId) -> Vec<NodeId> {
        self.incoming_edges(node)
            .into_iter()
            .filter_map(|edge| self.edge(edge).map(|edge| edge.source))
            .collect()
    }

    pub fn contains_node(&self, node: NodeId) -> bool {
        self.nodes.get(node.index()).is_some_and(Option::is_some)
    }

    pub fn contains_edge(&self, edge: EdgeId) -> bool {
        self.edges.get(edge.index()).is_some_and(Option::is_some)
    }

    pub fn node_count(&self) -> usize {
        self.live_nodes
    }

    pub fn edge_count(&self) -> usize {
        self.live_edges
    }

    pub fn is_empty(&self) -> bool {
        self.live_nodes == 0
    }

    fn notify(&mut self, event: GraphEvent) {
        let mut observers = std::mem::take(&mut self.observers);
        for observer in &mut observers {
            observer.on_change(self, &event);
        }
        self.observers = observers;
    }

    fn unlink_out_edge(
        &mut self,
        source: NodeId,
        target_edge: EdgeId,
        replacement: Option<EdgeId>,
    ) {
        let Some(source_node) = self.nodes[source.index()].as_mut() else {
            return;
        };

        if source_node.first_out == Some(target_edge) {
            source_node.first_out = replacement;
            return;
        }

        let mut current = source_node.first_out;
        while let Some(edge) = current {
            let next = self.edges[edge.index()].as_ref().and_then(Edge::next_out);
            if next == Some(target_edge) {
                if let Some(edge_ref) = self.edges[edge.index()].as_mut() {
                    edge_ref.next_out = replacement;
                }
                return;
            }
            current = next;
        }
    }

    fn unlink_in_edge(&mut self, target: NodeId, target_edge: EdgeId, replacement: Option<EdgeId>) {
        let Some(target_node) = self.nodes[target.index()].as_mut() else {
            return;
        };

        if target_node.first_in == Some(target_edge) {
            target_node.first_in = replacement;
            return;
        }

        let mut current = target_node.first_in;
        while let Some(edge) = current {
            let next = self.edges[edge.index()].as_ref().and_then(Edge::next_in);
            if next == Some(target_edge) {
                if let Some(edge_ref) = self.edges[edge.index()].as_mut() {
                    edge_ref.next_in = replacement;
                }
                return;
            }
            current = next;
        }
    }
}

impl<N, E> GraphView for Graph<N, E> {
    type Node = NodeId;

    fn nodes(&self) -> Vec<Self::Node> {
        self.nodes()
    }

    fn successors(&self, node: &Self::Node) -> Vec<Self::Node> {
        self.successors(*node)
    }
}
