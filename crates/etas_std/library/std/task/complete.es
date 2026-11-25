module std.task;



public flow join<A, B, effect E>(jobs: (() -> A ![E], () -> B ![E])) -> (A, B) ![E] {
    let (job_a, job_b) = jobs;
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        let task_a = start(job_a);
        let task_b = start(job_b);
        let value_a = await(task_a);
        let value_b = await(task_b);
        return (value_a, value_b);
    });
}

public flow join<A, B, C, effect E>(jobs: (() -> A ![E], () -> B ![E], () -> C ![E])) -> (A, B, C) ![E] {
    let (job_a, job_b, job_c) = jobs;
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        let task_a = start(job_a);
        let task_b = start(job_b);
        let task_c = start(job_c);
        let value_a = await(task_a);
        let value_b = await(task_b);
        let value_c = await(task_c);
        return (value_a, value_b, value_c);
    });
}

public flow join<A, B, C, D, effect E>(jobs: (() -> A ![E], () -> B ![E], () -> C ![E], () -> D ![E])) -> (A, B, C, D) ![E] {
    let (job_a, job_b, job_c, job_d) = jobs;
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        let task_a = start(job_a);
        let task_b = start(job_b);
        let task_c = start(job_c);
        let task_d = start(job_d);
        let value_a = await(task_a);
        let value_b = await(task_b);
        let value_c = await(task_c);
        let value_d = await(task_d);
        return (value_a, value_b, value_c, value_d);
    });
}

public flow join<T, effect E>(jobs: Array<() -> T ![E]>) -> Array<T> ![E] {
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        var tasks: Array<Task<T, ![E]>> = [];
        for job in jobs {
            tasks = tasks.push(start(job));
        }
        var output: Array<T> = [];
        for task in tasks {
            output = output.push(await(task));
        }
        return output;
    });
}

public flow collect<T, X, effect E>(jobs: Array<() -> T ![Error<X>, E]>) -> Array<Result<T, X>> ![E] {
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        var tasks: Array<Task<Result<T, X>, ![E]>> = [];
        for job in jobs {
            tasks = tasks.push(start(() => job()?));
        }
        var output: Array<Result<T, X>> = [];
        for task in tasks {
            output = output.push(await(task));
        }
        return output;
    });
}

public flow map_concurrent<A, B, effect E>(values: Array<A>, transform: (A) -> B ![E]) -> Array<B> ![E] {
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        var tasks: Array<Task<B, ![E]>> = [];
        for value in values {
            tasks = tasks.push(start(() => transform(value)));
        }
        var output: Array<B> = [];
        for task in tasks {
            output = output.push(await(task));
        }
        return output;
    });
}

public flow map_concurrent_collect<A, B, X, effect E>(values: Array<A>, transform: (A) -> B ![Error<X>, E])
    -> Array<Result<B, X>> ![E] {
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        var tasks: Array<Task<Result<B, X>, ![E]>> = [];
        for value in values {
            tasks = tasks.push(start(() => transform(value)?));
        }
        var output: Array<Result<B, X>> = [];
        for task in tasks {
            output = output.push(await(task));
        }
        return output;
    });
}

public flow try_join<T, X, effect E>(jobs: Array<() -> T ![Error<X>, E]>) -> Result<Array<T>, X> ![E] {
    return Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        var tasks: Array<Task<Result<T, X>, ![E]>> = [];
        var roots: std.collections.OrderedMap<u64, usize> = std.collections.OrderedMap.new();
        var slots: Array<Option<T>> = [];
        for job in jobs {
            let task: Task<Result<T, X>, ![E]> = start(() => job()?);
            roots = roots.insert(task.id, tasks.length);
            tasks = tasks.push(task);
            slots = slots.push(None);
        }
        var complete: usize = 0;
        var primary: Option<(usize, X)> = None;
        while complete < tasks.length {
            let event = scheduler.next_completion();
            let index = match roots.get(event.task) {
                Some(index) => index,
                // Nested tasks also produce events; this group observes roots.
                None => { continue; },
            };
            complete = complete + 1;
            // Cancellation also produces terminal events. Await only tasks
            // whose source Result has actually completed.
            if !event.completed { continue; }
            let result = await(tasks.get(index).unwrap());
            match result {
                Ok(value) => {
                    slots = slots.set(index, Some(value))?.unwrap();
                },
                Err(error) => {
                    let first = match primary {
                        Some(_) => false,
                        None => true,
                    };
                    match primary {
                        Some((previous, _)) => {
                            if index < previous { primary = Some((index, error)); }
                        },
                        None => { primary = Some((index, error)); },
                    };
                    if first {
                        for task in tasks {
                            scheduler.cancel(task);
                        }
                    }
                },
            };
        }
        // Drain cancellation before choosing the lowest input index among all
        // errors that reached a terminal state at the cancellation barrier.
        match primary {
            Some((_, error)) => { return Err(error); },
            None => {},
        };
        var output: Array<T> = [];
        for slot in slots {
            output = output.push(slot.unwrap());
        }
        return Ok(output);
    });
}

public flow race<T, X, effect E>(jobs: Array<() -> T ![Error<X>, E]>) -> T ![Error<X>, E] {
    if jobs.length == 0 { std.core.abort("race requires at least one job"); }
    let result: Result<T, X> = Scope.inherit((scheduler: std.task.scheduler.SchedulerHandle) => {
        var tasks: Array<Task<Result<T, X>, ![E]>> = [];
        var roots: std.collections.OrderedMap<u64, usize> = std.collections.OrderedMap.new();
        for job in jobs {
            let task: Task<Result<T, X>, ![E]> = start(() => job()?);
            roots = roots.insert(task.id, tasks.length);
            tasks = tasks.push(task);
        }
        var complete: usize = 0;
        var primary: Option<(usize, X)> = None;
        while complete < tasks.length {
            let event = scheduler.next_completion();
            let index = match roots.get(event.task) {
                Some(index) => index,
                None => { continue; },
            };
            let result = await(tasks.get(index).unwrap());
            match result {
                Ok(value) => {
                    for task in tasks {
                        scheduler.cancel(task);
                    }
                    return Ok(value);
                },
                Err(error) => {
                    match primary {
                        Some((previous, _)) => {
                            if index < previous { primary = Some((index, error)); }
                        },
                        None => { primary = Some((index, error)); },
                    };
                    complete = complete + 1;
                },
            };
        }
        let (_, error) = primary.unwrap();
        return Err(error);
    });
    return match result {
        Ok(value) => value,
        Err(error) => perform Error.raise<X>(error),
    };
}
