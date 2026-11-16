module std.task;

import std.task.scheduler.{SchedulerHandle, DefaultScheduler};

@std.role("std.task.Async")
public effect Async {
    action yield() -> unit;
    action<Output, effect Remaining> await(task: Task<Output, ![Remaining]>) -> Output;
}

@std.role("std.task.Task")
public type Task<T, effect E>;

@std.role("std.task.Scheduler")
public type Scheduler;

@std.role("std.task.Continuation")
public type Continuation<T>;

@std.binding("std.task.run")
// The selected policy must be deterministic and proven non-suspending; its
// scope-local copy is distinct from the active execution handle.
public flow run<T, effect E>(body: (SchedulerHandle) -> T ![E], implementation: ? ~ std.task.scheduler.Scheduler = DefaultScheduler) -> T ![E] {}

@std.binding("std.task.start")
public flow start<T, effect E>(body: () -> T ![E]) -> Task<T, ![E]> ![E] {}

public flow await<T, effect E>(task: Task<T, ![E]>) -> T ![E] {
    return perform Async.await(task);
}
