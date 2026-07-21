use std::{
    collections::{HashSet, VecDeque},
    hash::Hash,
};

#[derive(Clone, Debug)]
pub struct Worklist<N> {
    queue: VecDeque<N>,
    queued: HashSet<N>,
}

impl<N> Default for Worklist<N> {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
            queued: HashSet::new(),
        }
    }
}

impl<N> Worklist<N>
where
    N: Clone + Eq + Hash,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, node: N) -> bool {
        if !self.queued.insert(node.clone()) {
            return false;
        }
        self.queue.push_back(node);
        true
    }

    pub fn extend(&mut self, nodes: impl IntoIterator<Item = N>) {
        for node in nodes {
            self.push(node);
        }
    }

    pub fn pop(&mut self) -> Option<N> {
        let node = self.queue.pop_front()?;
        self.queued.remove(&node);
        Some(node)
    }

    pub fn contains(&self, node: &N) -> bool {
        self.queued.contains(node)
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.queued.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }
}

impl<N> FromIterator<N> for Worklist<N>
where
    N: Clone + Eq + Hash,
{
    fn from_iter<T: IntoIterator<Item = N>>(iter: T) -> Self {
        let mut worklist = Self::new();
        worklist.extend(iter);
        worklist
    }
}
