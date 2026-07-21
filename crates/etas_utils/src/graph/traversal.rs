use std::{
    collections::{HashSet, VecDeque},
    hash::Hash,
};

pub trait GraphView {
    type Node: Clone + Eq + Hash;

    fn nodes(&self) -> Vec<Self::Node>;

    fn successors(&self, node: &Self::Node) -> Vec<Self::Node>;
}

pub fn dfs<G>(graph: &G, roots: impl IntoIterator<Item = G::Node>) -> Vec<G::Node>
where
    G: GraphView,
{
    let mut visited = HashSet::new();
    let mut order = Vec::new();

    for root in roots {
        dfs_visit(graph, root, &mut visited, &mut order);
    }

    order
}

fn dfs_visit<G>(graph: &G, node: G::Node, visited: &mut HashSet<G::Node>, order: &mut Vec<G::Node>)
where
    G: GraphView,
{
    if !visited.insert(node.clone()) {
        return;
    }

    order.push(node.clone());
    for successor in graph.successors(&node) {
        dfs_visit(graph, successor, visited, order);
    }
}

pub fn bfs<G>(graph: &G, roots: impl IntoIterator<Item = G::Node>) -> Vec<G::Node>
where
    G: GraphView,
{
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut order = Vec::new();

    for root in roots {
        if visited.insert(root.clone()) {
            queue.push_back(root);
        }
    }

    while let Some(node) = queue.pop_front() {
        order.push(node.clone());
        for successor in graph.successors(&node) {
            if visited.insert(successor.clone()) {
                queue.push_back(successor);
            }
        }
    }

    order
}

pub fn reverse_postorder<G>(graph: &G, roots: impl IntoIterator<Item = G::Node>) -> Vec<G::Node>
where
    G: GraphView,
{
    let mut visited = HashSet::new();
    let mut postorder = Vec::new();

    for root in roots {
        reverse_postorder_visit(graph, root, &mut visited, &mut postorder);
    }

    postorder.reverse();
    postorder
}

fn reverse_postorder_visit<G>(
    graph: &G,
    node: G::Node,
    visited: &mut HashSet<G::Node>,
    postorder: &mut Vec<G::Node>,
) where
    G: GraphView,
{
    if !visited.insert(node.clone()) {
        return;
    }

    for successor in graph.successors(&node) {
        reverse_postorder_visit(graph, successor, visited, postorder);
    }
    postorder.push(node);
}
