#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainControl {
    Continue,
    Break,
}

pub trait ChainStep<C> {
    fn run(&mut self, context: &mut C) -> ChainControl;
}

impl<C, F> ChainStep<C> for F
where
    F: FnMut(&mut C) -> ChainControl,
{
    fn run(&mut self, context: &mut C) -> ChainControl {
        self(context)
    }
}

pub struct ResponsibilityChain<C> {
    steps: Vec<Box<dyn ChainStep<C>>>,
}

impl<C> Default for ResponsibilityChain<C> {
    fn default() -> Self {
        Self { steps: Vec::new() }
    }
}

impl<C> ResponsibilityChain<C> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, step: impl ChainStep<C> + 'static) {
        self.steps.push(Box::new(step));
    }

    pub fn run(&mut self, context: &mut C) -> ChainControl {
        for step in &mut self.steps {
            if step.run(context) == ChainControl::Break {
                return ChainControl::Break;
            }
        }
        ChainControl::Continue
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }
}
