use super::{Pipeline, UnitOrder, UnitSelector};

pub struct ForEachAdapter<C> {
    pub selector: UnitSelector,
    pub order: UnitOrder,
    pub pipeline: Pipeline<C>,
}

impl<C> ForEachAdapter<C> {
    pub fn new(selector: UnitSelector, order: UnitOrder, pipeline: Pipeline<C>) -> Self {
        Self {
            selector,
            order,
            pipeline,
        }
    }
}
