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

public type FsEntry;
public type FsStat = { is_file: bool, is_dir: bool, len: u64 };

public enum ReadLimitError { LimitExceeded(usize), }
public enum ListLimitError { LimitExceeded(usize, usize), }

@std.binding("std.fs.root")
public flow root<R ~ Region>() -> WorkspacePath<R> ![] {}

@std.binding("std.fs.relative_name")
public flow relative_name<R ~ Region>(path: WorkspacePath<R>) -> string ![] {}

@std.binding("std.fs.read_bytes_limited")
public flow read_bytes_limited<R ~ Region>(path: WorkspacePath<R>, max_bytes: usize) -> bytes ![Error<IOError>, Error<ReadLimitError>] {}

@std.binding("std.fs.write_bytes")
public flow write_bytes<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<IOError>] {}

@std.binding("std.fs.list")
public flow list<R ~ Region>(path: WorkspacePath<R>) -> List<WorkspacePath<R>> ![Error<IOError>] {}

@std.binding("std.fs.list_limited")
public flow list_limited<R ~ Region>(path: WorkspacePath<R>, max_entries: usize, max_bytes: usize) -> List<WorkspacePath<R>> ![Error<IOError>, Error<ListLimitError>] {}

@std.binding("std.fs.stat")
public flow stat<R ~ Region>(path: WorkspacePath<R>) -> FsStat ![Error<IOError>] {}

@std.binding("std.fs.atomic_replace")
public flow atomic_replace<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<IOError>] {}
