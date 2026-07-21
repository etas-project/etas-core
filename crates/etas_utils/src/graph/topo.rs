use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use super::GraphView;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cycle<N> {
    pub nodes: Vec<N>,
}

pub fn topological_sort<G>(graph: &G) -> Result<Vec<G::Node>, Cycle<G::Node>>
where
    G: GraphView,
{
    let mut colors = HashMap::new();
    let mut stack = Vec::new();
    let mut order = Vec::new();

    for node in graph.nodes() {
        if colors.get(&node).copied().unwrap_or(Color::White) == Color::White {
            visit(graph, node, &mut colors, &mut stack, &mut order)?;
        }
    }

    order.reverse();
    Ok(order)
}

pub fn cycle_nodes<G>(graph: &G) -> Option<Vec<G::Node>>
where
    G: GraphView,
{
    topological_sort(graph).err().map(|cycle| cycle.nodes)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Color {
    White,
    Gray,
    Black,
}

fn visit<G>(
    graph: &G,
    node: G::Node,
    colors: &mut HashMap<G::Node, Color>,
    stack: &mut Vec<G::Node>,
    order: &mut Vec<G::Node>,
) -> Result<(), Cycle<G::Node>>
where
    G: GraphView,
{
    colors.insert(node.clone(), Color::Gray);
    stack.push(node.clone());

    for successor in graph.successors(&node) {
        match colors.get(&successor).copied().unwrap_or(Color::White) {
            Color::White => visit(graph, successor, colors, stack, order)?,
            Color::Gray => {
                return Err(Cycle {
                    nodes: extract_cycle(stack, &successor),
                });
            }
            Color::Black => {}
        }
    }

    stack.pop();
    colors.insert(node.clone(), Color::Black);
    order.push(node);
    Ok(())
}

fn extract_cycle<N>(stack: &[N], repeated: &N) -> Vec<N>
where
    N: Clone + Eq + Hash,
{
    let mut seen = false;
    let mut cycle = Vec::new();
    for node in stack {
        if node == repeated {
            seen = true;
        }
        if seen {
            cycle.push(node.clone());
        }
    }
    cycle.push(repeated.clone());
    dedupe_consecutive(cycle)
}

fn dedupe_consecutive<N>(nodes: Vec<N>) -> Vec<N>
where
    N: Clone + Eq + Hash,
{
    let mut deduped = Vec::new();
    let mut seen = HashSet::new();
    for node in nodes {
        if seen.insert(node.clone()) {
            deduped.push(node);
        }
    }
    deduped
}
