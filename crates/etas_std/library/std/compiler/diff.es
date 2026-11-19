module std.compiler;

public flow diff<A>(old: AgentDef<A>, new: AgentDef<A>) -> ChangeReport ![] {
    let before = old.observation;
    let after = new.observation;
    if before.identity != after.identity || before.entry != after.entry {
        std.core.abort("diff requires compatible immutable definition identities");
    }
    return ChangeReport {
        schema= "etas.code.change-report.v2",
        identity= before.identity,
        old_revision= old.revision,
        new_revision= new.revision,
        old_body_available= before.body.is_some(),
        new_body_available= after.body.is_some(),
        body_changed= before.body.observation_difference(after.body),
        signature_changed= before.signature != after.signature,
        dependencies_changed= before.environment != after.environment,
        declared_bound_changed= before.declared_bound != after.declared_bound,
        escaping_changed= before.escaping.observation_difference(after.escaping),
        requested_changed= before.requested.observation_difference(after.requested),
        default_handled_changed= before.default_handled.observation_difference(after.default_handled),
        determinism_changed= before.determinism.observation_difference(after.determinism),
        trace_changed= before.trace.observation_difference(after.trace),
    };
}

public flow diff<F>(old: CallableDef<F>, new: CallableDef<F>) -> ChangeReport ![] {
    let before = old.observation;
    let after = new.observation;
    if before.identity != after.identity || before.entry != after.entry {
        std.core.abort("diff requires compatible immutable definition identities");
    }
    return ChangeReport {
        schema= "etas.code.change-report.v2",
        identity= before.identity,
        old_revision= old.revision,
        new_revision= new.revision,
        old_body_available= before.body.is_some(),
        new_body_available= after.body.is_some(),
        body_changed= before.body.observation_difference(after.body),
        signature_changed= before.signature != after.signature,
        dependencies_changed= before.environment != after.environment,
        declared_bound_changed= before.declared_bound != after.declared_bound,
        escaping_changed= before.escaping.observation_difference(after.escaping),
        requested_changed= before.requested.observation_difference(after.requested),
        default_handled_changed= before.default_handled.observation_difference(after.default_handled),
        determinism_changed= before.determinism.observation_difference(after.determinism),
        trace_changed= before.trace.observation_difference(after.trace),
    };
}

impl Option<T> {
    flow observation_difference(self, other: Option<T>) -> Option<bool> ![] {
        return match (self, other) {
            (Some(before), Some(after)) => Some(before != after),
            _ => None,
        };
    }
}
