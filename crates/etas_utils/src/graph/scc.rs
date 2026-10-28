use std::collections::{HashMap, HashSet};

use super::GraphView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Scc<N> {
    pub nodes: Vec<N>,
}

pub fn strongly_connected_components<G>(graph: &G) -> Vec<Scc<G::Node>>
where
    G: GraphView,
{
    let mut tarjan = Tarjan {
        graph,
        next_index: 0,
        index: HashMap::new(),
        lowlink: HashMap::new(),
        stack: Vec::new(),
        on_stack: HashSet::new(),
        components: Vec::new(),
    };

    for node in graph.nodes() {
        if !tarjan.index.contains_key(&node) {
            tarjan.visit(node);
        }
    }

    tarjan.components
}

struct Tarjan<'a, G>
where
    G: GraphView,
{
    graph: &'a G,
    next_index: usize,
    index: HashMap<G::Node, usize>,
    lowlink: HashMap<G::Node, usize>,
    stack: Vec<G::Node>,
    on_stack: HashSet<G::Node>,
    components: Vec<Scc<G::Node>>,
}

impl<G> Tarjan<'_, G>
where
    G: GraphView,
{
    fn visit(&mut self, node: G::Node) {
        self.discover(node.clone());
        let mut pending = vec![(node.clone(), self.graph.successors(&node).into_iter())];
        while let Some((node, successors)) = pending.last_mut() {
            if let Some(successor) = successors.next() {
                if !self.index.contains_key(&successor) {
                    self.discover(successor.clone());
                    let successors = self.graph.successors(&successor).into_iter();
                    pending.push((successor, successors));
                } else if self.on_stack.contains(&successor) {
                    let successor_index = self.index[&successor];
                    let node_lowlink = self.lowlink.get_mut(node).expect("node lowlink exists");
                    *node_lowlink = (*node_lowlink).min(successor_index);
                }
                continue;
            }
            let Some((node, _)) = pending.pop() else {
                break;
            };
            self.complete(&node);
            if let Some((parent, _)) = pending.last() {
                let successor_lowlink = self.lowlink[&node];
                let parent_lowlink = self.lowlink.get_mut(parent).expect("parent lowlink exists");
                *parent_lowlink = (*parent_lowlink).min(successor_lowlink);
            }
        }
    }

    fn discover(&mut self, node: G::Node) {
        let index = self.next_index;
        self.next_index += 1;
        self.index.insert(node.clone(), index);
        self.lowlink.insert(node.clone(), index);
        self.stack.push(node.clone());
        self.on_stack.insert(node.clone());
    }

    fn complete(&mut self, node: &G::Node) {
        if self.lowlink[node] == self.index[node] {
            let mut component = Vec::new();
            loop {
                let Some(member) = self.stack.pop() else {
                    break;
                };
                self.on_stack.remove(&member);
                component.push(member.clone());
                if &member == node {
                    break;
                }
            }
            self.components.push(Scc { nodes: component });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Chain {
        nodes: usize,
        cyclic: bool,
    }
    impl GraphView for Chain {
        type Node = usize;
        fn nodes(&self) -> Vec<usize> {
            (0..self.nodes).collect()
        }
        fn successors(&self, node: &usize) -> Vec<usize> {
            if *node + 1 < self.nodes {
                vec![*node + 1]
            } else if self.cyclic {
                vec![0]
            } else {
                vec![]
            }
        }
    }

    #[test]
    fn deep_dependency_chains_emit_callee_components_first_without_stack_recursion() {
        let components = strongly_connected_components(&Chain {
            nodes: 10_000,
            cyclic: false,
        });
        assert_eq!(components.len(), 10_000);
        for (index, component) in components.iter().enumerate() {
            assert_eq!(component.nodes, [9_999 - index]);
        }
    }

    #[test]
    fn deep_recursive_components_do_not_use_the_native_stack() {
        let components = strongly_connected_components(&Chain {
            nodes: 10_000,
            cyclic: true,
        });
        assert_eq!(components.len(), 1);
        let nodes = components[0].nodes.iter().copied().collect::<HashSet<_>>();
        assert_eq!(nodes.len(), 10_000);
    }
}
