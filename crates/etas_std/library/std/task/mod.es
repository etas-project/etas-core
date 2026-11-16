module std.task;

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
public flow run<T, effect E>(body: (Scheduler) -> T ![E]) -> T ![E] {}

@std.binding("std.task.start")
public flow start<T, effect E>(body: () -> T ![E]) -> Task<T, ![E]> ![E] {}

public flow await<T, effect E>(task: Task<T, ![E]>) -> T ![E] {
    return perform Async.await(task);
}
