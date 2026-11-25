module app.resource_scope;

effect Probe {
    action tick(value: i32) -> unit;
}

flow main(args: Array<string>) -> i32 ![] {
    var total = 0;
    let result = handle {
        std.resource.scope(() => {
            let child = std.task.start(() => {
                perform Probe.tick(7);
            });
            return 42;
        })?
    } with {
        Probe.tick(value) => {
            total = total + value;
            resume ();
        }
    };
    let value = match result {
        Ok(value) => value,
        Err(_) => std.core.abort("resource scope cleanup failed"),
    };
    assert(value == 42);
    assert(total == 7);
    let compute_result = std.resource.scope(() => {
        let base = 41;
        return () => { return base + 1; };
    })?;
    let compute = match compute_result {
        Ok(value) => value,
        Err(_) => std.core.abort("resource scope cleanup failed"),
    };
    assert(compute() == 42);
    return 0;
}
