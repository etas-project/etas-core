module std.task.scheduler;

import std.task.{Task, Scheduler, Continuation};

@std.binding("std.task.scheduler.park")
public flow park<T, effect E>(scheduler: Scheduler, task: Task<T, ![E]>, continuation: Continuation<T>) -> never ![] {}

@std.binding("std.task.scheduler.requeue")
public flow requeue<T>(scheduler: Scheduler, continuation: Continuation<T>, value: T) -> never ![] {}
