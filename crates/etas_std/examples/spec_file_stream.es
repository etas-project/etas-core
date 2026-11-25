module app.file_stream;

import std.effects.Fs;

type Files = {};
impl Files ~ std.fs.Region {}

flow main(args: Array<string>) -> i32 ![Error<std.io.IOError>, Error<std.stream.StreamError>, Error<std.resource.ResourceError>] {
    let path = std.fs.path<Files>("file-stream.txt").unwrap();
    return std.resource.scope(() => {
        let writer = std.fs.open(path, std.fs.FileMode.WriteTruncate);
        std.stream.write_all(writer, "hello".utf8);
        std.fs.sync(writer);
        std.stream.close(writer);

        let file = std.fs.open(path, std.fs.FileMode.ReadWrite);
        assert(std.stream.read_exact(file, 2) == "he".utf8);
        std.stream.write_all(file, "y".utf8);
        assert(std.stream.read_exact(file, 2) == "lo".utf8);
        std.stream.close(file);

        let reader = std.fs.open(path, std.fs.FileMode.Read);
        let alias = handle {
            std.fs.open(path, std.fs.FileMode.Read)
        } with {
            Fs.open<Files>(requested, mode) => {
                assert(mode == std.fs.FileMode.Read);
                resume reader;
            }
        };
        assert(std.stream.read_exact(alias, 5) == "heylo".utf8);
        std.stream.close(alias);
        assert(std.fs.relative_name(std.fs.canonicalize(path)) == "file-stream.txt");
        return 0;
    });
}
