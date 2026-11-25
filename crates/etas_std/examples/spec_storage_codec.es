module app.storage_codec;

type Counter = { value: i32 };

impl Counter {
    flow encode_next(var self) -> bytes ![] {
        self.value = self.value + 1;
        let codec = std.memory.codec<i32>();
        return match (codec.encode)(self.value) {
            Ok(encoded) => encoded,
            Err(_) => std.core.abort("expected a valid integer encoding"),
        };
    }
}

flow main(args: Array<string>) -> i32 ![] {
    var counter = Counter { value= 41 };
    let encoded = counter.encode_next();
    let codec = std.memory.codec<i32>();
    let decoded = match (codec.decode)(encoded) {
        Ok(value) => value,
        Err(_) => std.core.abort("expected a valid integer decoding"),
    };
    std.core.assert(counter.value == 42);
    std.core.assert(decoded == 42);
    return 0;
}
