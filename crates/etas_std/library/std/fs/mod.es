module std.fs;

public import std.io.IOError;

public spec Region;

public enum FileMode {
    Read,
    WriteTruncate,
    Append,
    ReadWrite,
    CreateNew,
}

type FileConnection;
public type FileStream<R ~ Region> = { private connection: FileConnection };
impl FileStream<R ~ Region> ~ std.stream.ByteStream {}

// A source-owned relative locator. R remains the checked authority selector;
// no path value contains a host root or grants access to that region.
public type WorkspacePath<R ~ Region> = { private relative: string };

public type FsEntry = {
    name: string,
    kind: FsEntryKind,
};

public enum FsEntryKind { File, Directory, Symlink, Other, }
public type FsStat = { is_file: bool, is_dir: bool, len: u64 };
public enum ReadLimitError { LimitExceeded(usize), }
public enum ListLimitError { LimitExceeded(usize, usize), }

public flow path<R ~ Region>(relative: string) -> Result<WorkspacePath<R>, IOError> ![] {
    if relative == "." { return Ok(WorkspacePath { relative= "." }); }
    if std.text.contains(relative, "\\") {
        return Err(IOError.InvalidInput);
    }
    let raw = relative.utf8;
    if raw.length >= 2 {
        let first = raw.get(0).unwrap();
        if ((first >= 65 && first <= 90) || (first >= 97 && first <= 122)) && raw.get(1).unwrap() == 58 {
            return Err(IOError.InvalidInput);
        }
    }
    if std.text.starts_with(relative, "/") {
        return Err(IOError.InvalidInput);
    }
    var normalized = "";
    for part in std.text.split(relative, "/") {
        if part == ".." { return Err(IOError.InvalidInput); }
        if part != "" && part != "." {
            if normalized != "" { normalized = normalized + "/"; }
            normalized = normalized + part;
        }
    }
    if normalized == "" { return Err(IOError.InvalidInput); }
    return Ok(WorkspacePath { relative= normalized });
}

public flow root<R ~ Region>() -> WorkspacePath<R> ![] {
    return WorkspacePath { relative= "." };
}

public flow relative_name<R ~ Region>(path: WorkspacePath<R>) -> string ![] {
    return path.relative;
}

public flow read_bytes<R ~ Region>(path: WorkspacePath<R>) -> bytes ![Error<IOError>] {
    return (perform std.effects.Fs.read<R>(path, None)).unwrap();
}

public flow read_bytes_limited<R ~ Region>(path: WorkspacePath<R>, max_bytes: usize) -> bytes ![Error<IOError>] {
    return match perform std.effects.Fs.read<R>(path, Some(max_bytes)) {
        Ok(body) => body,
        Err(_) => perform Error.raise<IOError>(IOError.LimitExceeded),
    };
}

public flow write_bytes<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<IOError>] {
    perform std.effects.Fs.write<R>(path, contents);
}

public flow list<R ~ Region>(path: WorkspacePath<R>) -> List<WorkspacePath<R>> ![Error<IOError>] {
    let entries = (perform std.effects.Fs.list<R>(path, None)).unwrap();
    var ordered: std.collections.OrderedMap<string, WorkspacePath<R>> = std.collections.OrderedMap.new();
    for entry in entries {
        ordered = ordered.insert(entry.relative, entry);
    }
    var output: List<WorkspacePath<R>> = [];
    for (name, entry) in ordered.iter() {
        output = entry :: output;
    }
    return output.reverse();
}

public flow list_limited<R ~ Region>(path: WorkspacePath<R>, max_entries: usize, max_bytes: usize) -> List<WorkspacePath<R>> ![Error<IOError>] {
    let entries = match perform std.effects.Fs.list<R>(path, Some((max_entries, max_bytes))) {
        Ok(values) => values,
        Err(_) => perform Error.raise<IOError>(IOError.LimitExceeded),
    };
    var ordered: std.collections.OrderedMap<string, WorkspacePath<R>> = std.collections.OrderedMap.new();
    for entry in entries {
        ordered = ordered.insert(entry.relative, entry);
    }
    var output: List<WorkspacePath<R>> = [];
    for (name, entry) in ordered.iter() {
        output = entry :: output;
    }
    return output.reverse();
}

public flow stat<R ~ Region>(path: WorkspacePath<R>) -> FsStat ![Error<IOError>] {
    return perform std.effects.Fs.stat<R>(path);
}

public flow atomic_replace<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<IOError>] {
    perform std.effects.Fs.atomic_replace<R>(path, contents);
}

public flow create_dir<R ~ Region>(path: WorkspacePath<R>, recursive: bool = false) -> unit ![Error<IOError>] {
    perform std.effects.Fs.create_dir<R>(path, recursive);
}

public flow rename<R ~ Region>(source: WorkspacePath<R>, destination: WorkspacePath<R>) -> unit ![Error<IOError>] {
    perform std.effects.Fs.rename<R>(source, destination);
}

public flow remove_file<R ~ Region>(path: WorkspacePath<R>) -> unit ![Error<IOError>] {
    perform std.effects.Fs.remove_file<R>(path);
}

public flow remove_dir<R ~ Region>(path: WorkspacePath<R>) -> unit ![Error<IOError>] {
    perform std.effects.Fs.remove_dir<R>(path);
}

public flow open<R ~ Region>(path: WorkspacePath<R>, mode: FileMode) -> FileStream<R> ![Error<IOError>] {
    return perform std.effects.Fs.open<R>(path, mode);
}

public flow sync<R ~ Region>(file: FileStream<R>) -> unit ![Error<IOError>] {
    perform std.effects.Fs.sync<R>(file);
}

public flow canonicalize<R ~ Region>(path: WorkspacePath<R>) -> WorkspacePath<R> ![Error<IOError>] {
    return perform std.effects.Fs.canonicalize<R>(path);
}
