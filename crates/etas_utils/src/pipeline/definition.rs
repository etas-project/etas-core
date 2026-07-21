use super::{ForEachAdapter, Pass, UnitOrder, UnitSelector};

pub struct Pipeline<C> {
    pub name: &'static str,
    pub steps: Vec<PipelineStep<C>>,
}

impl<C> Pipeline<C> {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            steps: Vec::new(),
        }
    }

    pub fn push_pass<P>(&mut self, pass: P)
    where
        P: Pass<C> + 'static,
    {
        self.steps.push(PipelineStep::Pass(Box::new(pass)));
    }

    pub fn push_group(&mut self, group: Pipeline<C>) {
        self.steps.push(PipelineStep::Group(group));
    }

    pub fn push_for_each(
        &mut self,
        selector: UnitSelector,
        order: UnitOrder,
        pipeline: Pipeline<C>,
    ) {
        self.steps.push(PipelineStep::ForEach(ForEachAdapter::new(
            selector, order, pipeline,
        )));
    }

    pub fn pass<P>(mut self, pass: P) -> Self
    where
        P: Pass<C> + 'static,
    {
        self.push_pass(pass);
        self
    }

    pub fn group(mut self, group: Pipeline<C>) -> Self {
        self.push_group(group);
        self
    }

    pub fn for_each(
        mut self,
        selector: UnitSelector,
        order: UnitOrder,
        pipeline: Pipeline<C>,
    ) -> Self {
        self.push_for_each(selector, order, pipeline);
        self
    }
}

pub enum PipelineStep<C> {
    Pass(Box<dyn Pass<C>>),
    Group(Pipeline<C>),
    ForEach(ForEachAdapter<C>),
}
