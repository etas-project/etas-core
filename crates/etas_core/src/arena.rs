use std::ops::Index;

use crate::Idx;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Arena<Id, T> {
    values: Vec<T>,
    _id: std::marker::PhantomData<fn() -> Id>,
}

impl<Id, T> Default for Arena<Id, T> {
    fn default() -> Self {
        Self {
            values: Vec::new(),
            _id: std::marker::PhantomData,
        }
    }
}

impl<Id, T> Arena<Id, T>
where
    Id: Idx,
{
    pub fn alloc(&mut self, value: T) -> Id {
        let id = Id::from_u32(self.values.len().min(u32::MAX as usize) as u32);
        self.values.push(value);
        id
    }

    pub fn alloc_with_id(&mut self, make_value: impl FnOnce(Id) -> T) -> Id {
        let id = Id::from_u32(self.values.len().min(u32::MAX as usize) as u32);
        self.values.push(make_value(id));
        id
    }

    pub fn iter(&self) -> impl Iterator<Item = (Id, &T)> {
        self.values
            .iter()
            .enumerate()
            .map(|(index, value)| (Id::from_u32(index.min(u32::MAX as usize) as u32), value))
    }

    pub fn get(&self, id: Id) -> Option<&T> {
        self.values.get(id.into_usize())
    }

    pub fn get_mut(&mut self, id: Id) -> Option<&mut T> {
        self.values.get_mut(id.into_usize())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

impl<Id, T> Index<Id> for Arena<Id, T>
where
    Id: Idx,
{
    type Output = T;

    fn index(&self, index: Id) -> &Self::Output {
        &self.values[index.into_usize()]
    }
}
