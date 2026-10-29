use crate::{ActionTrace, DefaultHandlerEffects, EffectRow, GenericParam, Type};

mod behavior;
mod paths;
mod validation;
pub use behavior::*;

fn validate_request_target(
    target: &crate::CallableRequestTarget,
    labels: &std::collections::BTreeSet<&str>,
    root: Option<(usize, &[crate::GenericParamKind])>,
    depth: usize,
) -> Result<(), crate::MetadataArtifactError> {
    target.validate_shape().map_err(|reason| invalid(&reason))?;
    match target {
        crate::CallableRequestTarget::Parameter(label) => {
            if !labels.contains(label.as_str()) {
                return Err(invalid("callable request has an unbound parameter"));
            }
        }
        crate::CallableRequestTarget::SpecMethod(invocation) => {
            for argument in &invocation.callable_arguments {
                argument
                    .sources
                    .validate_references_at_depth(root, depth + 1)?;
            }
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallableValuePathKind {
    Type,
    Effect,
    Resource,
    Spec,
}

/// A lexical callable scope inside one metadata graph, never an executable ID.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum CallableValueScope {
    Root,
    Value(u32),
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct CallableValueParameter {
    pub scope: CallableValueScope,
    pub position: u32,
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub enum CallableValueSource {
    Parameter(CallableValueParameter),
    Value(u32),
}

/// Finite analysis data. References may be cyclic; nodes do not embed other nodes.
/// None at the containing summary means unavailable evidence, not an empty graph.
#[derive(
    Clone,
    Debug,
    Default,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    etas_core::RetainedSize,
)]
pub struct CallableValueGraph {
    pub sources: Vec<CallableValueSource>,
    pub values: Vec<CallableValueNode>,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct CallableValueParameterBinding {
    /// A trace-local label, resolved only through this table, not by name lookup.
    pub label: String,
    pub parameter: CallableValueParameter,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct CallableValueGenericBinding {
    /// The type-variable / row-tail label in this node's structured descriptors.
    pub label: String,
    pub scope: CallableValueScope,
    pub position: u32,
}

#[derive(
    Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, etas_core::RetainedSize,
)]
pub struct CallableValueNode {
    pub handler_requests: Option<crate::RequestInstantiationTemplate>,
    pub behavior: CallableValueBehavior,
    pub parent_scope: CallableValueScope,
    pub generic_params: Vec<GenericParam>,
    pub generic_bindings: Vec<CallableValueGenericBinding>,
    pub param_names: Vec<String>,
    pub params: Vec<Type>,
    pub output: Type,
    pub public_effects: EffectRow,
    pub requested_actions: EffectRow,
    pub handled_requested_actions: EffectRow,
    pub default_handler_effects: DefaultHandlerEffects,
    pub parameter_bindings: Vec<CallableValueParameterBinding>,
    pub action_trace: ActionTrace,
    pub returned_sources: Option<Vec<CallableValueSource>>,
}

impl CallableValueGraph {
    /// Validates identity references without recursively expanding the graph.
    /// The frontend supplies the checked root interface before using the graph.
    pub fn validate_references(
        &self,
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        self.validate_references_at_depth(root, 0)
    }

    pub(crate) fn validate_references_at_depth(
        &self,
        root: Option<(usize, &[crate::GenericParamKind])>,
        depth: usize,
    ) -> Result<(), crate::MetadataArtifactError> {
        if depth > crate::MAX_METADATA_GRAPH_DEPTH {
            return Err(invalid(
                "nested callable evidence exceeds the maximum depth",
            ));
        }
        use std::collections::BTreeSet;
        self.validate_sources(&self.sources, root)?;
        for source in &self.sources {
            self.validate_source_scope(*source, &BTreeSet::from([CallableValueScope::Root]))?;
        }
        for (index, node) in self.values.iter().enumerate() {
            if node
                .behavior
                .host_requirements
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                != node.behavior.host_requirements.len()
                || node
                    .behavior
                    .interpreter_features
                    .iter()
                    .collect::<BTreeSet<_>>()
                    .len()
                    != node.behavior.interpreter_features.len()
            {
                return Err(invalid("callable behavior contains duplicate requirements"));
            }
            let mut scopes = BTreeSet::from([CallableValueScope::Root]);
            let mut scope = CallableValueScope::Value(
                u32::try_from(index).map_err(|_| invalid("callable graph index exceeds u32"))?,
            );
            while let CallableValueScope::Value(index) = scope {
                if !scopes.insert(scope) {
                    return Err(invalid("callable value lexical scopes contain a cycle"));
                }
                if scopes.len() > crate::MAX_METADATA_GRAPH_DEPTH + 1 {
                    return Err(invalid(
                        "callable value lexical scopes exceed the maximum depth",
                    ));
                }
                scope = self.value(index)?.parent_scope;
            }
            self.validate_descriptor_bindings(node, root)?;
            if node.param_names.len() != node.params.len()
                || node.param_names.iter().any(String::is_empty)
                || node.param_names.iter().collect::<BTreeSet<_>>().len() != node.param_names.len()
            {
                return Err(invalid("callable value has an invalid parameter interface"));
            }
            let mut generics = BTreeSet::new();
            for generic in &node.generic_params {
                if generic.name.is_empty() || !generics.insert(&generic.name) {
                    return Err(invalid(
                        "callable value has duplicate or empty generic parameters",
                    ));
                }
            }
            let mut labels = BTreeSet::new();
            for binding in &node.generic_bindings {
                if !scopes.contains(&binding.scope) {
                    return Err(invalid(
                        "callable generic binding escapes its lexical scope",
                    ));
                }
                if binding.label.is_empty() || !labels.insert(&binding.label) {
                    return Err(invalid(
                        "callable value has duplicate or empty generic binding labels",
                    ));
                }
                self.generic_parameter(binding.scope, binding.position, root)?;
            }
            let mut labels = BTreeSet::new();
            for binding in &node.parameter_bindings {
                if !scopes.contains(&binding.parameter.scope) {
                    return Err(invalid(
                        "callable parameter binding escapes its lexical scope",
                    ));
                }
                if binding.label.is_empty() || !labels.insert(binding.label.as_str()) {
                    return Err(invalid(
                        "callable value has duplicate or empty trace parameter labels",
                    ));
                }
                self.parameter(binding.parameter, root)?;
            }
            let mut trace = vec![&node.action_trace];
            if let Some(requests) = &node.handler_requests {
                for parameter in &requests.parameters {
                    validate_request_target(&parameter.parameter, &labels, root, depth)?;
                }
            }
            while let Some(part) = trace.pop() {
                match part {
                    ActionTrace::ParameterCall { parameter } => {
                        validate_request_target(parameter, &labels, root, depth)?
                    }
                    ActionTrace::Widened {
                        parameter_calls, ..
                    } => {
                        for parameter in parameter_calls {
                            validate_request_target(parameter, &labels, root, depth)?;
                        }
                    }
                    ActionTrace::Seq(parts) | ActionTrace::Choice(parts) => trace.extend(parts),
                    ActionTrace::Repeat(part) => trace.push(part),
                    _ => {}
                }
            }
            if let Some(sources) = &node.returned_sources {
                self.validate_sources(sources, root)?;
                for source in sources {
                    self.validate_source_scope(*source, &scopes)?;
                }
            }
        }
        Ok(())
    }

    fn validate_source_scope(
        &self,
        source: CallableValueSource,
        scopes: &std::collections::BTreeSet<CallableValueScope>,
    ) -> Result<(), crate::MetadataArtifactError> {
        let scope = match source {
            CallableValueSource::Parameter(parameter) => parameter.scope,
            CallableValueSource::Value(index) => self.value(index)?.parent_scope,
        };
        if !scopes.contains(&scope) {
            return Err(invalid(
                "returned callable source escapes its lexical scope",
            ));
        }
        Ok(())
    }

    fn validate_sources(
        &self,
        sources: &[CallableValueSource],
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        let mut seen = std::collections::BTreeSet::new();
        if sources.is_empty() {
            return Err(invalid("returned callable source evidence is empty"));
        }
        for source in sources {
            if !seen.insert(source) {
                return Err(invalid("returned callable source evidence is duplicated"));
            }
            match source {
                CallableValueSource::Parameter(parameter) => self.parameter(*parameter, root)?,
                CallableValueSource::Value(index) => {
                    self.value(*index)?;
                }
            }
        }
        Ok(())
    }

    fn value(&self, index: u32) -> Result<&CallableValueNode, crate::MetadataArtifactError> {
        self.values
            .get(index as usize)
            .ok_or_else(|| invalid("returned callable value reference is out of range"))
    }

    fn parameter(
        &self,
        parameter: CallableValueParameter,
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        let count = match parameter.scope {
            CallableValueScope::Root => root.map(|(count, _)| count),
            CallableValueScope::Value(index) => Some(self.value(index)?.params.len()),
        };
        if count.is_some_and(|count| parameter.position as usize >= count) {
            return Err(invalid(
                "returned callable parameter reference is not declared by its scope",
            ));
        }
        Ok(())
    }

    fn generic_parameter(
        &self,
        scope: CallableValueScope,
        position: u32,
        root: Option<(usize, &[crate::GenericParamKind])>,
    ) -> Result<(), crate::MetadataArtifactError> {
        let count = match scope {
            CallableValueScope::Root => root.map(|(_, generics)| generics.len()),
            CallableValueScope::Value(index) => Some(self.value(index)?.generic_params.len()),
        };
        if count.is_some_and(|count| position as usize >= count) {
            return Err(invalid(
                "callable generic reference is not declared by its scope",
            ));
        }
        Ok(())
    }
}

fn invalid(message: &str) -> crate::MetadataArtifactError {
    crate::MetadataArtifactError::invalid("<callable-values>", message)
}
