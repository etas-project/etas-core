import std.command.{Command, run};

flow main(args: Array<string>) -> i32 ![] {
    let initial: Map<string, string> = Map.new();
    let environment = initial.insert("ETAS_STD_UNUSED", "removed")
        .insert("ETAS_STD_MAP", "before")
        .insert("ETAS_STD_MAP", "etas")
        .remove("ETAS_STD_UNUSED");
    let command = Command.new("/usr/bin/printenv", ["ETAS_STD_MAP"]).with_env(environment);
    let result = run(command);
    std.core.assert(result.exit_code == 0);
    std.core.assert(result.stdout == "etas\n".utf8);
    return 0;
}
