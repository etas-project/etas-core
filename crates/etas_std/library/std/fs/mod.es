module std.fs;

// The compiler uses Region as the static selector for filesystem authority.
@std.role("std.fs.Region")
public spec Region;

// The host owns path storage and enforces the selected region at each operation.
@std.role("std.fs.WorkspacePath")
public type WorkspacePath<R ~ Region>;

// Ordinary source declaration; native bindings carry its opaque error payload.
public type IOError;

@std.binding("std.fs.path")
public flow path<R ~ Region>(relative: string) -> Result<WorkspacePath<R>, IOError> ![] {}

@std.binding("std.fs.read_bytes")
public flow read_bytes<R ~ Region>(path: WorkspacePath<R>) -> bytes ![Error<IOError>] {}
