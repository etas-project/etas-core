module app.storage_store;

import std.memory.{Store, StorageError, WriteCondition, WriteOutcome, ReconcileResult, ConfirmedOutcome};

type Sample = { serial: u128, reading: f32, labels: Array<string> };
alias Schema = { Samples: Store<u128, Sample> };
let Region = std.memory.region<Schema>(stable_id= "storage-store-example", store= "example");

flow main(args: Array<string>) -> i32 ![Error<StorageError>] {
    let key: u128 = 340282366920938463463374607431768211455;
    let sample = Sample { serial= key, reading= f32 { bits= 2143289345 }, labels= ["binary", "source"] };
    let store = Region.Samples;
    let intent = store.prepare_put(key, sample, WriteCondition.Any);
    let operation = intent.operation_ref();
    let receipt = match intent.commit() {
        WriteOutcome.Committed(receipt) => receipt,
        _ => std.core.abort("expected a committed example write"),
    };
    std.core.assert(receipt.operation == operation);
    std.core.assert(receipt.target.key == key);

    let restored = store.get(key).unwrap();
    std.core.assert(restored.serial == key);
    std.core.assert(restored.reading.bits == 2143289345);
    std.core.assert(restored.labels == ["binary", "source"]);
    let recovered = match store.reconcile(operation) {
        ReconcileResult.Found(ConfirmedOutcome.Committed(value)) => value,
        _ => std.core.abort("expected retained receipt evidence"),
    };
    std.core.assert(recovered.operation == receipt.operation);
    std.core.assert(recovered.target.key == key);

    let view = store.with_codecs(std.memory.key_codec<u128>(), std.memory.codec<Sample>());
    std.core.assert(view.get_entry(key).unwrap().version == store.get_entry(key).unwrap().version);
    let keys = view.keys();
    std.core.assert(keys.len() == 1);
    std.core.assert(keys.head().unwrap() == key);
    let selected = view.scan().take(1).collect();
    std.core.assert(selected.length == 1);
    std.core.assert(selected.get(0).unwrap().reading.bits == 2143289345);
    store.clear();
    std.core.assert(!view.contains(key));
    return 0;
}
