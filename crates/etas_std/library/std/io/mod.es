module std.io;

// The host owns the opaque failure payload; the public type belongs to source.
public type IOError;

@std.binding("std.io.read_all")
public flow read_all() -> string ![Error<IOError>] {}

@std.binding("std.io.read_line")
public flow read_line() -> string ![Error<IOError>] {}

@std.binding("std.io.print")
public flow print(text: string) -> unit ![Error<IOError>] {}

public flow println(text: string) -> unit ![Error<IOError>] {
    print(text + "\n");
}

@std.binding("std.io.eprintln")
public flow eprintln(text: string) -> unit ![Error<IOError>] {}
