module app.http_stream;

import std.http.codec.{HttpResponseDecoder, HttpResponseHeadDecoder, HttpDecodeStep};

flow main(args: Array<string>) -> i32 {
    let head = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n".utf8;
    let chunks = "2;a=\"b\\c\"\r\nab\r\n3\r\ncde\r\n0\r\nX-End: yes\r\n\r\n".utf8;
    let message = head + chunks;
    var headers = HttpResponseHeadDecoder.new();
    var response = HttpResponseDecoder.new();
    var size: usize = 1;
    while size < message.length {
        let prefix = message[0, size)?.unwrap();
        if size <= head.length {
            match headers.decode(prefix, false) {
                HttpDecodeStep.NeedMore => std.core.assert(size < head.length),
                HttpDecodeStep.Complete(value, end) => std.core.assert(value.status == 200 && end == head.length),
                _ => { return 1; },
            };
        }
        match response.decode(prefix, false) {
            HttpDecodeStep.NeedMore => {},
            _ => { return 2; },
        };
        size = size + 1;
    }
    return match response.decode(message + "next".utf8, false) {
        HttpDecodeStep.Complete(value, end) => if value.body == "abcde".utf8 && end == message.length { 0 } else { 3 },
        _ => 4,
    };
}
