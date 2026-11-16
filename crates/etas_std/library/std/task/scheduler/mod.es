module std.task.scheduler;

@std.binding("std.task.scheduler.park")
public flow park<T, effect E>(scheduler: std.task.Scheduler, task: std.task.Task<T, ![E]>, continuation: std.task.Continuation<T>) -> never ![] {}

@std.binding("std.task.scheduler.requeue")
public flow requeue<T>(scheduler: std.task.Scheduler, continuation: std.task.Continuation<T>, value: T) -> never ![] {}
