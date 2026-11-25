import std.text.TextBuilder;

flow main(args: Array<string>) -> i32 ![] {
    let values: Array<i32> = [5, 1, 4, 2, 3];
    let selected = values.iter().filter((value: i32) => value % 2 == 1).map((value: i32) => value * 10).to_array();
    let initial: Map<string, i32> = Map.new();
    let replaced = initial.insert("beta", 2).insert("alpha", 1).insert("beta", 20);
    let keys = replaced.keys().to_array();
    let compare = (left: i32, right: i32) => left.compare(right);
    let first: OrderedSet<i32> = OrderedSet.new(compare);
    let second: OrderedSet<i32> = OrderedSet.new(compare);
    let merged = first.insert(3).insert(1).union(second.insert(2).insert(3))?.unwrap().iter().to_array();
    var builder = TextBuilder.new();
    builder.append("Etas");
    let saved = builder;
    builder.append(" std");
    let single: Array<f32> = [2.0, 3.0];
    let double: Array<f64> = [1.5, 2.5];
    let (_, median) = std.collections.select_nth(values, 2)?.unwrap();
    std.core.assert(selected == [50, 10, 30]);
    std.core.assert(keys == ["beta", "alpha"]);
    std.core.assert(merged == [1, 2, 3]);
    std.core.assert(saved.build() == "Etas");
    std.core.assert(builder.build() == "Etas std");
    std.core.assert(single.iter().product() == 6.0);
    std.core.assert(double.iter().sum() == 4.0);
    std.core.assert(std.collections.top_k(values.iter(), 3) == [5, 4, 3]);
    std.core.assert(median == 3);
    let rounds = std.agent.group.round_robin(
        [(state: (i32, () -> i32 ![])) => {
            let (value, callback) = state;
            return (value + 1, callback);
        }],
        (0, () => 42), 5,
        (state: (i32, () -> i32 ![])) => {
            let (value, _) = state;
            return value >= 3;
        },
    )?.unwrap();
    std.core.assert(rounds.turns == 3);
    let (_, callback) = rounds.state;
    std.core.assert(callback() == 42);
    return 0;
}
