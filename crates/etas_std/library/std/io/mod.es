module std.io;

public type IOError = { code: string, message: string };

public flow read_all() -> string ![Error<IOError>] {
    return perform std.effects.Console.stdin_read_all();
}

public flow read_line() -> string ![Error<IOError>] {
    return perform std.effects.Console.stdin_read_line();
}

public flow print(text: string) -> unit ![Error<IOError>] {
    perform std.effects.Console.stdout_write(text);
}

public flow println(text: string) -> unit ![Error<IOError>] {
    perform std.effects.Console.stdout_write(text + "\n");
}

public flow eprint(text: string) -> unit ![Error<IOError>] {
    perform std.effects.Console.stderr_write(text);
}

public flow eprintln(text: string) -> unit ![Error<IOError>] {
    perform std.effects.Console.stderr_write(text + "\n");
}
