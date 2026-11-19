module std.fs;

public spec Region;

// A source-owned relative locator. R remains the checked authority selector;
// no path value contains a host root or grants access to that region.
public type WorkspacePath<R ~ Region> = { private relative: string };

public type IOError = string;
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
        return Err(IOError("drive, UNC and backslash workspace paths are not allowed"));
    }
    let raw = relative.utf8;
    if raw.length >= 2 {
        let first = raw.get(0).unwrap();
        if ((first >= 65 && first <= 90) || (first >= 97 && first <= 122)) && raw.get(1).unwrap() == 58 {
            return Err(IOError("drive, UNC and backslash workspace paths are not allowed"));
        }
    }
    if std.text.starts_with(relative, "/") {
        return Err(IOError("absolute workspace paths are not allowed"));
    }
    var normalized = "";
    for part in std.text.split(relative, "/") limit Iterations(18446744073709551615) {
        if part == ".." { return Err(IOError("workspace path traversal is not allowed")); }
        if part != "" && part != "." {
            if normalized != "" { normalized = normalized + "/"; }
            normalized = normalized + part;
        }
    }
    if normalized == "" { return Err(IOError("workspace path must not be empty")); }
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

public flow read_bytes_limited<R ~ Region>(path: WorkspacePath<R>, max_bytes: usize) -> bytes ![Error<IOError>, Error<ReadLimitError>] {
    return match perform std.effects.Fs.read<R>(path, Some(max_bytes)) {
        Ok(body) => body,
        Err(error) => perform Error.raise<ReadLimitError>(error),
    };
}

public flow write_bytes<R ~ Region>(path: WorkspacePath<R>, contents: bytes) -> unit ![Error<IOError>] {
    perform std.effects.Fs.write<R>(path, contents);
}

public flow list<R ~ Region>(path: WorkspacePath<R>) -> List<WorkspacePath<R>> ![Error<IOError>] {
    let entries = (perform std.effects.Fs.list<R>(path, None)).unwrap();
    var ordered: std.collections.OrderedMap<string, WorkspacePath<R>> = std.collections.OrderedMap.new();
    for entry in entries limit Iterations(18446744073709551615) {
        ordered = ordered.insert(entry.relative, entry);
    }
    var output: List<WorkspacePath<R>> = [];
    for (name, entry) in ordered.iter() limit Iterations(18446744073709551615) {
        output = entry :: output;
    }
    return output.reverse();
}

public flow list_limited<R ~ Region>(path: WorkspacePath<R>, max_entries: usize, max_bytes: usize) -> List<WorkspacePath<R>> ![Error<IOError>, Error<ListLimitError>] {
    let entries = match perform std.effects.Fs.list<R>(path, Some((max_entries, max_bytes))) {
        Ok(values) => values,
        Err(error) => perform Error.raise<ListLimitError>(error),
    };
    var ordered: std.collections.OrderedMap<string, WorkspacePath<R>> = std.collections.OrderedMap.new();
    for entry in entries limit Iterations(18446744073709551615) {
        ordered = ordered.insert(entry.relative, entry);
    }
    var output: List<WorkspacePath<R>> = [];
    for (name, entry) in ordered.iter() limit Iterations(18446744073709551615) {
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
