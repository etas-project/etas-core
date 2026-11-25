module app.storage_outcomes;

import std.memory.{Store, StorageError, StorageOperationRef, WriteOutcome, MemoryWriteRejection};
import std.effects.Memory;

alias Schema = { Items: Store<string, string> };
let Region = std.memory.region<Schema>(stable_id= "storage-outcomes-example", store= "example");

flow main(args: Array<string>) -> i32 ![Error<StorageError>] {
    var submissions: usize = 0;
    let absent = handler {
        Memory.write(store, operation, request, wire) => {
            submissions = submissions + 1;
            resume WriteOutcome.NotCommitted(operation, MemoryWriteRejection.Unchanged);
        }
    };
    Region.Items.delete("absent") with absent;
    std.core.assert(submissions == 1);

    let uncertain = handler {
        Memory.write(store, operation, request, wire) => {
            submissions = submissions + 1;
            resume WriteOutcome.Unknown(operation);
        }
    };
    var recovery: Option<StorageOperationRef> = None;
    handle {
        Region.Items.put("key", "value") with uncertain;
    } with {
        Error<StorageError>.raise(error) => {
            std.core.assert(error.code == "outcome_unknown");
            recovery = error.operation;
            finish ();
        }
    };
    std.core.assert(submissions == 2);
    std.core.assert(recovery.is_some());
    return 0;
}
