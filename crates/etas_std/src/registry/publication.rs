use super::{StdRegistry, StdSymbolId};
use crate::{StdDecl, StdEffectRef, StdGenericParam, StdStaticArg, StdType};
use std::collections::BTreeSet;

impl StdRegistry {
    /// Build the native part of an installed source provider. Stable IDs remain
    /// unchanged; unbound bootstrap declarations cannot become public fallbacks.
    pub fn for_source_bindings(mut self, bindings: &BTreeSet<StdSymbolId>) -> Self {
        self.execution_roles.clear();
        self.parameter_interfaces.clear();
        let mut names = Vec::new();
        for symbol in self
            .symbols()
            .filter(|symbol| bindings.contains(&symbol.id))
        {
            names.push(symbol.qualified_path.join("."));
            declaration_names(&symbol.decl, &mut names);
        }
        let mut used = BTreeSet::new();
        while let Some(name) = names.pop() {
            let path = self
                .native_type_reference(&name)
                .map(|reference| reference.path.join("."))
                .unwrap_or(name);
            if !used.insert(path.clone()) {
                continue;
            }
            if let Some(reference) = self.native_record_references.get(&path) {
                for field in &reference.fields {
                    type_names(&field.ty, &mut names);
                }
            }
            if let Some(reference) = self.native_projection_references.get(&path) {
                for field in &reference.fields {
                    type_names(&field.ty, &mut names);
                }
            }
        }
        for symbol in &mut self.symbols {
            if symbol
                .as_ref()
                .is_some_and(|symbol| !bindings.contains(&symbol.id))
            {
                *symbol = None;
            }
        }
        self.qualified.retain(|_, symbol| bindings.contains(symbol));
        let removed = self
            .prelude
            .symbols()
            .filter_map(|symbol| (!bindings.contains(&symbol.id)).then_some(symbol.id))
            .collect();
        self.prelude.remove_symbols(&removed);
        let intrinsics = self
            .symbols()
            .filter_map(|symbol| symbol.intrinsic.as_ref().map(|value| value.id))
            .collect::<BTreeSet<_>>();
        self.intrinsics.retain(|id, _| intrinsics.contains(id));
        self.memory_place_results
            .retain(|id, _| intrinsics.contains(id));
        self.declarations
            .retain(|symbol, _| bindings.contains(symbol));
        self.flow_overloads
            .retain(|symbol, _| bindings.contains(symbol));
        self.flow_overload_bodies
            .retain(|symbol, _| bindings.contains(symbol));
        self.action_overloads
            .retain(|symbol, _| bindings.contains(symbol));
        self.enum_field_names
            .retain(|symbol, _| bindings.contains(symbol));
        self.native_type_references
            .retain(|path, _| used.contains(path));
        self.native_enum_references
            .retain(|path, _| used.contains(path));
        self.native_opaque_references
            .retain(|path, _| used.contains(path));
        self.native_record_references
            .retain(|path, _| used.contains(path));
        self.native_projection_references
            .retain(|path, _| used.contains(path));
        self
    }
}

fn parameter_names(parameters: &[StdGenericParam], names: &mut Vec<String>) {
    for parameter in parameters {
        for bound in &parameter.bounds {
            names.push(bound.path.join("."));
            for arg in &bound.args {
                type_names(arg, names);
            }
        }
    }
}
fn effect_names(effects: &[StdEffectRef], names: &mut Vec<String>) {
    for effect in effects {
        names.push(effect.path.join("."));
        for arg in &effect.args {
            if let StdStaticArg::Type(ty) = arg {
                type_names(ty, names);
            }
        }
    }
}
fn declaration_names(declaration: &StdDecl, names: &mut Vec<String>) {
    match declaration {
        StdDecl::Type(value) => {
            parameter_names(&value.params, names);
            if let Some(ty) = &value.representation {
                type_names(ty, names);
            }
            if let Some(ty) = &value.canonical_type {
                type_names(ty, names);
            }
            for field in &value.projected_fields {
                type_names(&field.ty, names);
            }
        }
        StdDecl::Flow(value) => {
            parameter_names(&value.type_params, names);
            for ty in &value.params {
                type_names(ty, names);
            }
            type_names(&value.output, names);
            effect_names(&value.public_effects, names);
            effect_names(&value.requested_actions, names);
        }
        StdDecl::EffectAction(value) => {
            parameter_names(&value.type_params, names);
            for ty in &value.params {
                type_names(ty, names);
            }
            type_names(&value.output, names);
            if let Some(effects) = &value.default_handler_effects {
                effect_names(effects, names);
            }
        }
        StdDecl::Effect(value) => parameter_names(&value.params, names),
        StdDecl::Requirement(value) => {
            for ty in &value.params {
                type_names(ty, names);
            }
        }
        StdDecl::Value(value) => type_names(&value.ty, names),
        StdDecl::Tool(_) => {
            unreachable!("compiler bootstrap contains no external tool declarations")
        }
    }
}
fn type_names(ty: &StdType, names: &mut Vec<String>) {
    match ty {
        StdType::Existential { bounds } => {
            for bound in bounds {
                names.push(bound.path.join("."));
                for arg in &bound.args {
                    type_names(arg, names);
                }
            }
        }
        StdType::Handler {
            handled,
            produced,
            result,
        } => {
            for row in std::iter::once(handled).chain(produced.iter()) {
                effect_names(&row.effects, names);
            }
            if let Some(result) = result {
                type_names(result, names);
            }
        }
        StdType::Record(fields) => {
            for field in fields {
                type_names(&field.ty, names);
            }
        }
        StdType::Named(name) => names.push(name.clone()),
        StdType::NamedApplied { name, args } => {
            names.push(name.clone());
            for ty in args {
                type_names(ty, names);
            }
        }
        StdType::Array(ty)
        | StdType::List(ty)
        | StdType::Set(ty)
        | StdType::Range(ty)
        | StdType::Slice(ty)
        | StdType::Option(ty)
        | StdType::Schema(ty)
        | StdType::Message(ty)
        | StdType::MemorySelection(ty)
        | StdType::MemoryRegion(ty)
        | StdType::ResourceHandleMemoryRegion(ty)
        | StdType::Trust { inner: ty, .. } => type_names(ty, names),
        StdType::Map { key, value } | StdType::Store { key, value } => {
            type_names(key, names);
            type_names(value, names);
        }
        StdType::Result { ok, err } => {
            type_names(ok, names);
            type_names(err, names);
        }
        StdType::Tuple(types) => {
            for ty in types {
                type_names(ty, names);
            }
        }
        StdType::Function {
            input,
            output,
            effects,
        } => {
            for ty in input {
                type_names(ty, names);
            }
            type_names(output, names);
            effect_names(&effects.effects, names);
        }
        StdType::Task { output, effects } => {
            type_names(output, names);
            effect_names(&effects.effects, names);
        }
        StdType::Primitive(_)
        | StdType::Var(_)
        | StdType::Support(_)
        | StdType::Prompt
        | StdType::PromptPart => {}
    }
}
