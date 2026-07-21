use std::collections::{BTreeMap, BTreeSet};

use super::{Matcher, ProductTraceStep, State as AutomatonState, SymbolicAutomaton};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProductState<Component, State> {
    components: BTreeMap<Component, State>,
}

impl<Component, State> Default for ProductState<Component, State> {
    fn default() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }
}

impl<Component, State> ProductState<Component, State>
where
    Component: Clone + Ord,
    State: Clone,
{
    pub fn new(components: BTreeMap<Component, State>) -> Self {
        Self { components }
    }

    pub fn components(&self) -> &BTreeMap<Component, State> {
        &self.components
    }

    pub fn get(&self, component: &Component) -> Option<&State> {
        self.components.get(component)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductStepResult<Component, State, Label> {
    pub states: BTreeSet<ProductState<Component, State>>,
    pub trace: Vec<ProductTraceStep<Component, State, Label>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError<Component> {
    MissingComponent { component: Component },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductAutomaton<Component, State, Label> {
    components: BTreeMap<Component, SymbolicAutomaton<State, Label>>,
}

impl<Component, State, Label> Default for ProductAutomaton<Component, State, Label> {
    fn default() -> Self {
        Self {
            components: BTreeMap::new(),
        }
    }
}

impl<Component, State, Label> ProductAutomaton<Component, State, Label>
where
    Component: Clone + Ord,
    State: AutomatonState,
{
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, component: Component, automaton: SymbolicAutomaton<State, Label>) {
        self.components.insert(component, automaton);
    }

    pub fn components(&self) -> &BTreeMap<Component, SymbolicAutomaton<State, Label>> {
        &self.components
    }

    pub fn initial_state(&self) -> ProductState<Component, State> {
        ProductState::new(
            self.components
                .iter()
                .map(|(component, automaton)| (component.clone(), automaton.initial().clone()))
                .collect(),
        )
    }

    pub fn step<Event, M>(
        &self,
        state: &ProductState<Component, State>,
        event: &Event,
        matcher: &M,
    ) -> Result<ProductStepResult<Component, State, Label>, StepError<Component>>
    where
        Label: Clone,
        M: Matcher<Label, Event>,
    {
        let mut products = vec![BTreeMap::new()];
        let mut trace = Vec::new();

        for (component, automaton) in &self.components {
            let Some(current) = state.get(component).cloned() else {
                return Err(StepError::MissingComponent {
                    component: component.clone(),
                });
            };
            let step = automaton.step(&current, event, matcher);
            trace.extend(step.trace.into_iter().map(|step| ProductTraceStep {
                component: component.clone(),
                step,
            }));

            let mut next_products = Vec::new();
            for product in &products {
                for next_state in &step.states {
                    let mut next_product = product.clone();
                    next_product.insert(component.clone(), next_state.clone());
                    next_products.push(next_product);
                }
            }
            products = next_products;
        }

        Ok(ProductStepResult {
            states: products.into_iter().map(ProductState::new).collect(),
            trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::ProductAutomaton;
    use crate::automaton::{Matcher, NoTransitionPolicy, SymbolicAutomaton};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct EqMatcher;

    impl Matcher<&'static str, &'static str> for EqMatcher {
        fn matches(&self, label: &&'static str, event: &&'static str) -> bool {
            label == event
        }
    }

    #[test]
    fn product_step_updates_each_component() {
        let mut left = SymbolicAutomaton::new("left.start", NoTransitionPolicy::Stay);
        left.add_transition("left.start", "left.done", "go");
        let mut right = SymbolicAutomaton::new("right.start", NoTransitionPolicy::Stay);
        right.add_transition("right.start", "right.done", "go");

        let mut product = ProductAutomaton::new();
        product.insert("left", left);
        product.insert("right", right);

        let result = product
            .step(&product.initial_state(), &"go", &EqMatcher)
            .expect("product state has all components");
        let state = result
            .states
            .iter()
            .next()
            .expect("one deterministic state");

        assert_eq!(state.get(&"left"), Some(&"left.done"));
        assert_eq!(state.get(&"right"), Some(&"right.done"));
    }

    #[test]
    fn product_step_rejects_missing_component_state() {
        let left = SymbolicAutomaton::new("left.start", NoTransitionPolicy::Stay);
        let right = SymbolicAutomaton::new("right.start", NoTransitionPolicy::Stay);

        let mut product = ProductAutomaton::new();
        product.insert("left", left);
        product.insert("right", right);

        let mut partial = std::collections::BTreeMap::new();
        partial.insert("left", "left.start");
        let err = product
            .step(&super::ProductState::new(partial), &"go", &EqMatcher)
            .expect_err("missing right component must fail closed");

        assert_eq!(
            err,
            super::StepError::MissingComponent { component: "right" }
        );
    }
}
