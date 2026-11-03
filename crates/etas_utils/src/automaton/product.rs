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
    pub accepting: BTreeSet<ProductState<Component, State>>,
    pub rejecting: BTreeSet<ProductState<Component, State>>,
    pub trace: Vec<ProductTraceStep<Component, State, Label>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductStateClassification {
    pub accepting: bool,
    pub rejecting: bool,
    pub work: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StepError<Component> {
    MissingComponent { component: Component },
    UnexpectedComponent { component: Component },
    UnknownState { component: Component },
    ComponentLimit,
    StateLimit,
    WorkLimit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProductStepLimits {
    pub max_components: usize,
    pub max_product_states: usize,
    pub max_work: usize,
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

    /// Validates a complete product state and checks every component's exit
    /// classification under one explicit work budget.
    pub fn classify_state_bounded(
        &self,
        state: &ProductState<Component, State>,
        max_components: usize,
        max_work: usize,
    ) -> Result<ProductStateClassification, StepError<Component>> {
        if self.components.len() > max_components || state.components.len() > max_components {
            return Err(StepError::ComponentLimit);
        }
        let mut work = state.components.len();
        if work > max_work {
            return Err(StepError::WorkLimit);
        }
        for component in state.components.keys() {
            if !self.components.contains_key(component) {
                return Err(StepError::UnexpectedComponent {
                    component: component.clone(),
                });
            }
        }
        let mut accepting = true;
        let mut rejecting = false;
        for (component, automaton) in &self.components {
            let current = state
                .get(component)
                .ok_or_else(|| StepError::MissingComponent {
                    component: component.clone(),
                })?;
            work = work
                .checked_add(automaton.transitions().len())
                .and_then(|value| value.checked_add(1))
                .ok_or(StepError::WorkLimit)?;
            if work > max_work {
                return Err(StepError::WorkLimit);
            }
            if !automaton.contains_state(current) {
                return Err(StepError::UnknownState {
                    component: component.clone(),
                });
            }
            accepting &= automaton.accepting_states().contains(current);
            rejecting |= automaton.rejecting_states().contains(current);
        }
        Ok(ProductStateClassification {
            accepting,
            rejecting,
            work,
        })
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
        self.step_bounded(
            state,
            event,
            matcher,
            ProductStepLimits {
                max_components: usize::MAX,
                max_product_states: usize::MAX,
                max_work: usize::MAX,
            },
        )
    }

    /// Bounds component traversal, state combinations and map-copy work before
    /// allocating the next product. Use this entry point for untrusted plans.
    pub fn step_bounded<Event, M>(
        &self,
        state: &ProductState<Component, State>,
        event: &Event,
        matcher: &M,
        limits: ProductStepLimits,
    ) -> Result<ProductStepResult<Component, State, Label>, StepError<Component>>
    where
        Label: Clone,
        M: Matcher<Label, Event>,
    {
        if self.components.len() > limits.max_components
            || state.components.len() > limits.max_components
        {
            return Err(StepError::ComponentLimit);
        }
        if limits.max_product_states == 0 {
            return Err(StepError::StateLimit);
        }
        for component in state.components.keys() {
            if !self.components.contains_key(component) {
                return Err(StepError::UnexpectedComponent {
                    component: component.clone(),
                });
            }
        }
        let mut products = vec![(BTreeMap::new(), true, false)];
        let mut trace = Vec::new();
        let mut work = 0usize;

        for (index, (component, automaton)) in self.components.iter().enumerate() {
            let Some(current) = state.get(component).cloned() else {
                return Err(StepError::MissingComponent {
                    component: component.clone(),
                });
            };
            let transition_work = automaton
                .transitions()
                .len()
                .checked_mul(2)
                .ok_or(StepError::WorkLimit)?;
            work = work
                .checked_add(transition_work)
                .ok_or(StepError::WorkLimit)?;
            if work > limits.max_work {
                return Err(StepError::WorkLimit);
            }
            if !automaton.contains_state(&current) {
                return Err(StepError::UnknownState {
                    component: component.clone(),
                });
            }
            let step = automaton.step(&current, event, matcher);
            trace.extend(step.trace.into_iter().map(|step| ProductTraceStep {
                component: component.clone(),
                step,
            }));

            let count = products
                .len()
                .checked_mul(step.states.len())
                .ok_or(StepError::StateLimit)?;
            if count > limits.max_product_states {
                return Err(StepError::StateLimit);
            }
            let copy_work = count
                .checked_mul(index.saturating_add(1))
                .ok_or(StepError::WorkLimit)?;
            work = work.checked_add(copy_work).ok_or(StepError::WorkLimit)?;
            if work > limits.max_work {
                return Err(StepError::WorkLimit);
            }
            let mut next_products = Vec::with_capacity(count);
            for (product, accepting, rejecting) in &products {
                for next_state in &step.states {
                    let mut next_product = product.clone();
                    next_product.insert(component.clone(), next_state.clone());
                    next_products.push((
                        next_product,
                        *accepting && step.accepting.contains(next_state),
                        *rejecting || step.rejecting.contains(next_state),
                    ));
                }
            }
            products = next_products;
        }

        let mut states = BTreeSet::new();
        let mut accepting = BTreeSet::new();
        let mut rejecting = BTreeSet::new();
        for (components, accepts, rejects) in products {
            let state = ProductState::new(components);
            states.insert(state.clone());
            if accepts {
                accepting.insert(state.clone());
            }
            if rejects {
                rejecting.insert(state);
            }
        }
        Ok(ProductStepResult {
            states,
            accepting,
            rejecting,
            trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{ProductAutomaton, ProductStepLimits, StepError};
    use crate::automaton::{Matcher, NoTransitionPolicy, SymbolicAutomaton};

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct EqMatcher;

    impl Matcher<&'static str, &'static str> for EqMatcher {
        fn matches(&self, label: &&'static str, event: &&'static str) -> bool {
            label == event
        }
    }

    #[test]
    fn exit_classification_checks_components_acceptance_and_budget() {
        let mut first = SymbolicAutomaton::new("start", NoTransitionPolicy::Stay);
        first.add_transition("start", "done", "go");
        first.mark_accepting("done");
        first.mark_rejecting("bad");
        let mut second = SymbolicAutomaton::new("ready", NoTransitionPolicy::Stay);
        second.mark_accepting("ready");
        let mut product = ProductAutomaton::new();
        product.insert("first", first);
        product.insert("second", second);
        let initial = product.initial_state();
        let classification = product.classify_state_bounded(&initial, 2, 8).unwrap();
        assert!(!classification.accepting);
        assert!(!classification.rejecting);
        let step = product.step(&initial, &"go", &EqMatcher).unwrap();
        let done = step.states.iter().next().unwrap();
        let classification = product.classify_state_bounded(done, 2, 8).unwrap();
        assert!(classification.accepting);
        assert!(!classification.rejecting);
        let rejected = super::ProductState::new(std::collections::BTreeMap::from([
            ("first", "bad"),
            ("second", "ready"),
        ]));
        assert!(
            product
                .classify_state_bounded(&rejected, 2, 8)
                .unwrap()
                .rejecting
        );
        let missing =
            super::ProductState::new(std::collections::BTreeMap::from([("first", "start")]));
        assert!(matches!(
            product.classify_state_bounded(&missing, 2, 8),
            Err(StepError::MissingComponent { .. })
        ));
        let unknown = super::ProductState::new(std::collections::BTreeMap::from([
            ("first", "forged"),
            ("second", "ready"),
        ]));
        assert!(matches!(
            product.classify_state_bounded(&unknown, 2, 8),
            Err(StepError::UnknownState { .. })
        ));
        assert_eq!(
            product.classify_state_bounded(&initial, 2, 4),
            Err(StepError::WorkLimit)
        );
        assert_eq!(
            product.classify_state_bounded(&initial, 1, 8),
            Err(StepError::ComponentLimit)
        );
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

    #[test]
    fn product_step_rejects_unexpected_components_and_unknown_states() {
        let mut product = ProductAutomaton::new();
        product.insert(
            "left",
            SymbolicAutomaton::new("ready", NoTransitionPolicy::Stay),
        );
        let extra = super::ProductState::new(std::collections::BTreeMap::from([
            ("left", "ready"),
            ("extra", "ready"),
        ]));
        assert_eq!(
            product.step(&extra, &"event", &EqMatcher),
            Err(StepError::UnexpectedComponent { component: "extra" }),
        );
        let unknown =
            super::ProductState::new(std::collections::BTreeMap::from([("left", "forged")]));
        assert_eq!(
            product.step(&unknown, &"event", &EqMatcher),
            Err(StepError::UnknownState { component: "left" }),
        );
    }

    #[test]
    fn product_preserves_explicit_and_no_transition_rejections() {
        let mut rejected_transition = SymbolicAutomaton::new("start", NoTransitionPolicy::Stay);
        rejected_transition.add_transition("start", "bad", "deny");
        rejected_transition.mark_rejecting("bad");
        let unmatched_reject = SymbolicAutomaton::new("ready", NoTransitionPolicy::Reject);
        let mut product = ProductAutomaton::new();
        product.insert("explicit", rejected_transition);
        product.insert("unmatched", unmatched_reject);
        let step = product
            .step(&product.initial_state(), &"deny", &EqMatcher)
            .unwrap();
        let state = step.states.iter().next().unwrap();
        assert_eq!(state.get(&"explicit"), Some(&"bad"));
        assert_eq!(state.get(&"unmatched"), Some(&"ready"));
        assert!(step.rejecting.contains(state));

        let step = product
            .step(&product.initial_state(), &"unmatched", &EqMatcher)
            .unwrap();
        let state = step.states.iter().next().unwrap();
        assert_eq!(state.get(&"explicit"), Some(&"start"));
        assert!(
            step.rejecting.contains(state),
            "no-transition rejection must not disappear"
        );
    }

    #[test]
    fn product_accepts_only_when_every_component_accepts() {
        let mut left = SymbolicAutomaton::new("ready", NoTransitionPolicy::Stay);
        left.mark_accepting("ready");
        let mut right = SymbolicAutomaton::new("waiting", NoTransitionPolicy::Stay);
        right.add_transition("waiting", "ready", "go");
        right.mark_accepting("ready");
        let mut product = ProductAutomaton::new();
        product.insert("left", left);
        product.insert("right", right);
        let initial = product
            .step(&product.initial_state(), &"other", &EqMatcher)
            .unwrap();
        assert!(initial.accepting.is_empty());
        let completed = product
            .step(&product.initial_state(), &"go", &EqMatcher)
            .unwrap();
        assert_eq!(completed.accepting, completed.states);
        assert!(completed.rejecting.is_empty());
    }

    #[test]
    fn bounded_product_rejects_excessive_components_states_and_work() {
        let branching = || {
            let mut component = SymbolicAutomaton::new("start", NoTransitionPolicy::Stay);
            component.add_transition("start", "first", "go");
            component.add_transition("start", "second", "go");
            component
        };
        let mut product = ProductAutomaton::new();
        product.insert("left", branching());
        product.insert("right", branching());
        let initial = product.initial_state();
        let mut limits = ProductStepLimits {
            max_components: 2,
            max_product_states: 4,
            max_work: 18,
        };
        assert_eq!(
            product
                .step_bounded(&initial, &"go", &EqMatcher, limits)
                .unwrap()
                .states
                .len(),
            4
        );
        limits.max_product_states = 3;
        assert_eq!(
            product.step_bounded(&initial, &"go", &EqMatcher, limits),
            Err(StepError::StateLimit)
        );
        limits.max_product_states = 4;
        limits.max_work = 17;
        assert_eq!(
            product.step_bounded(&initial, &"go", &EqMatcher, limits),
            Err(StepError::WorkLimit)
        );
        limits.max_work = 18;
        limits.max_components = 1;
        assert_eq!(
            product.step_bounded(&initial, &"go", &EqMatcher, limits),
            Err(StepError::ComponentLimit)
        );
    }
}
