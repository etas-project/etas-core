module std.io;

public enum IOError {
    Eof,
    Interrupted,
    PermissionDenied,
    InvalidUtf8,
    Host(string),
    NotFound,
    AlreadyExists,
    InvalidInput,
    NotDirectory,
    IsDirectory,
    DirectoryNotEmpty,
    Closed,
    TimedOut,
    LimitExceeded,
    Unsupported,
}
