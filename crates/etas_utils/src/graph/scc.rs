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
        let index = self.next_index;
        self.next_index += 1;
        self.index.insert(node.clone(), index);
        self.lowlink.insert(node.clone(), index);
        self.stack.push(node.clone());
        self.on_stack.insert(node.clone());

        for successor in self.graph.successors(&node) {
            if !self.index.contains_key(&successor) {
                self.visit(successor.clone());
                let successor_lowlink = self.lowlink[&successor];
                let node_lowlink = self.lowlink.get_mut(&node).expect("node lowlink exists");
                *node_lowlink = (*node_lowlink).min(successor_lowlink);
            } else if self.on_stack.contains(&successor) {
                let successor_index = self.index[&successor];
                let node_lowlink = self.lowlink.get_mut(&node).expect("node lowlink exists");
                *node_lowlink = (*node_lowlink).min(successor_index);
            }
        }

        if self.lowlink[&node] == self.index[&node] {
            let mut component = Vec::new();
            loop {
                let Some(member) = self.stack.pop() else {
                    break;
                };
                self.on_stack.remove(&member);
                component.push(member.clone());
                if member == node {
                    break;
                }
            }
            self.components.push(Scc { nodes: component });
        }
    }
}
