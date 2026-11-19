module std.task;

import std.task.scheduler.{SchedulerHandle, DefaultScheduler};


@std.role("std.task.Async")
public effect Async {
    @default_handler
    action yield() -> unit ![];
    @default_handler
    action<Output, effect Remaining> await(task: Task<Output, ![Remaining]>) -> Output ![Remaining];
}

// These private handles denote live VM state, not ordinary collection storage.
// Source owns the public wrappers; execution primitives alone create handles.
type TaskHandle;
type SchedulerState;
type ContinuationState;

public type Task<T, effect E> = {
    private handle: TaskHandle,
    private id: u64,
};

public type Scheduler = {
    private state: SchedulerState,
};

public type Continuation<T> = {
    private state: ContinuationState,
};

// The selected policy must be deterministic and proven non-suspending; its
// scope-local copy is distinct from the active execution handle.
// The compiler checks the fresh owner lifetime; execution uses this source body.
public flow run<T, effect E>(body: (SchedulerHandle) -> T ![E], implementation: ? ~ std.task.scheduler.Scheduler = DefaultScheduler) -> T ![E] {
    return Scope.enter(implementation, (scheduler: Scheduler) => {
        let adapter = std.task.scheduler.handlers(scheduler);
        return handle {
            body(scheduler)
        } with adapter;
    }, false);
}
@std.role("std.task.start")
public flow start<T, effect E>(body: () -> T ![E]) -> Task<T, ![E]> ![E];
public flow await<T, effect E>(task: Task<T, ![E]>) -> T ![E] {
    return perform Async.await(task);
}
