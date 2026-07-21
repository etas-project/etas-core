use std::collections::BTreeSet;

pub trait PartialOrder {
    fn less_equal(&self, other: &Self) -> bool;

    fn equivalent(&self, other: &Self) -> bool {
        self.less_equal(other) && other.less_equal(self)
    }
}

pub trait JoinSemiLattice: PartialOrder {
    fn bottom() -> Self;

    fn join_assign(&mut self, other: &Self) -> bool;
}

pub trait MeetSemiLattice: PartialOrder {
    fn top() -> Self;

    fn meet_assign(&mut self, other: &Self) -> bool;
}

pub trait Lattice: JoinSemiLattice + MeetSemiLattice {}

impl<T> Lattice for T where T: JoinSemiLattice + MeetSemiLattice {}

impl PartialOrder for bool {
    fn less_equal(&self, other: &Self) -> bool {
        !*self || *other
    }
}

impl JoinSemiLattice for bool {
    fn bottom() -> Self {
        false
    }

    fn join_assign(&mut self, other: &Self) -> bool {
        let next = *self || *other;
        let changed = next != *self;
        *self = next;
        changed
    }
}

impl MeetSemiLattice for bool {
    fn top() -> Self {
        true
    }

    fn meet_assign(&mut self, other: &Self) -> bool {
        let next = *self && *other;
        let changed = next != *self;
        *self = next;
        changed
    }
}

impl<T> PartialOrder for BTreeSet<T>
where
    T: Ord,
{
    fn less_equal(&self, other: &Self) -> bool {
        self.is_subset(other)
    }
}

impl<T> JoinSemiLattice for BTreeSet<T>
where
    T: Clone + Ord,
{
    fn bottom() -> Self {
        Self::new()
    }

    fn join_assign(&mut self, other: &Self) -> bool {
        let old_len = self.len();
        self.extend(other.iter().cloned());
        self.len() != old_len
    }
}
