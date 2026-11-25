module app.storage_hash;

import std.collections.{Eq, Hash, Hasher, Map};

type Key = { value: u32 };

impl Key ~ Eq {}

impl Key ~ Hash {
    public flow hash(self, state: Hasher) -> Hasher ![] {
        let framed = "app.storage_hash.Key".hash(state);
        return self.value.hash(framed);
    }
}

type Counter = { count: usize };

impl Counter {
    flow roundtrip(var self) -> unit ![] {
        let initial: Map<Key, string> = Map.new();
        let values = initial.insert(Key { value= 7 }, "seven");
        let codec = std.memory.codec<Map<Key, string>>();
        let encoded = match (codec.encode)(values) {
            Ok(encoded) => encoded,
            Err(_) => std.core.abort("expected an encoded map"),
        };
        let restored = match (codec.decode)(encoded) {
            Ok(values) => values,
            Err(_) => std.core.abort("expected a restored map"),
        };
        std.core.assert(restored.get(Key { value= 7 }) == Some("seven"));
        self.count = restored.len();
    }
}

flow main(args: Array<string>) -> i32 ![] {
    var counter = Counter { count= 0 };
    counter.roundtrip();
    std.core.assert(counter.count == 1);
    return 0;
}
