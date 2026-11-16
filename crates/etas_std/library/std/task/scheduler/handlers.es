module std.task.scheduler;

public flow handlers(scope: SchedulerHandle) -> ![std.task.Async => []] ![] {
    return handler {
        std.task.Async.yield() as continuation => { requeue(scope, continuation, ()); }
        std.task.Async.await(job) as continuation => { park(scope, job, continuation); }
    };
}

impl std.task.Scheduler {
    public flow requeue<T>(self, continuation: std.task.Continuation<T>, value: T) -> never ![] {
        return std.task.scheduler.requeue(self, continuation, value);
    }
    public flow park<T, effect E>(self, task: std.task.Task<T, ![E]>, continuation: std.task.Continuation<T>) -> never ![] {
        return std.task.scheduler.park(self, task, continuation);
    }
}
