import std.collections.Hasher;
import std.memory.{StorageData, StorageKey, MemoryQuery, StorageError};

type Item<T> = { name: string, value: T };
enum Tree<T> {
    Leaf(T),
    Branch(Array<Tree<T>>),
}

flow admitted<T ~ StorageData>(value: T) -> T ![] {
    return value;
}

flow key<K ~ StorageKey>(value: K) -> K ![] {
    return value;
}

flow generic<T ~ StorageData>(value: Item<T>) -> Item<T> ![] {
    return admitted(value);
}

flow generic_key<K ~ StorageKey>(value: Item<K>) -> Item<K> ![] {
    return key(value);
}

flow main(args: Array<string>) -> i32 ![] {
    let large: u128 = 340282366920938463463374607431768211455;
    let value = admitted({ count= large, sample= f32 { bits= 2143289345 } });
    std.core.assert(value.count == large);
    std.core.assert(value.sample.bits == 2143289345);
    let item = key(Item { name= "first", value= 42 });
    std.core.assert(item == Item { value= 42, name= "first" });
    std.core.assert(generic_key(item) == item);
    let other = generic(Item { name= "second", value= [1, 2, 3] });
    std.core.assert(other.value == [1, 2, 3]);
    let tree: Tree<i32> = Tree.Branch([Tree.Leaf(1), Tree.Branch([Tree.Leaf(2)])]);
    std.core.assert(key(tree) == tree);
    let source: Array<i32> = [0, 1, 2, 3];
    let view: Slice<i32> = admitted(source[1, 3)?.unwrap());
    std.core.assert(view.to_array() == [1, 2]);
    let hasher = Hasher.new(7).write("source metadata".utf8);
    std.core.assert(admitted(hasher).finish() == hasher.finish());
    let query: MemoryQuery<i32> = admitted(MemoryQuery {
        language= "demo",
        version= 1,
        payload= "encoded".utf8,
    });
    std.core.assert(query.version == 1);
    let error = admitted(StorageError { code= "example", message= "data", operation= None });
    std.core.assert(error.code == "example");
    return 0;
}
