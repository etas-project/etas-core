module std.task;

@std.role("std.task.Async")
public effect Async {
    action yield() -> unit;
}

@std.role("std.task.Task")
public type Task<T, effect E>;

@std.role("std.task.Scheduler")
public type Scheduler;

@std.role("std.task.Continuation")
public type Continuation<T>;

@std.binding("std.task.run")
public flow run<T, effect E>(body: (Scheduler) -> T ![E]) -> T ![E] {}
