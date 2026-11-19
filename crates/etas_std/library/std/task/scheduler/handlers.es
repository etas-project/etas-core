module std.task.scheduler;

public flow handlers(scope: SchedulerHandle) -> ![std.task.Async => []] ![] {
    return handler {
        std.task.Async.yield() as continuation => {
            scope.requeue(continuation, ());
        }
        std.task.Async.await(job) as continuation => {
            scope.park(job, continuation);
        }
    };
}
