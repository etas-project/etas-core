module std.host.url;

public type Url = {
    scheme: string,
    username: Option<string>,
    password: Option<string>,
    host: Option<string>,
    port: Option<u16>,
    path: string,
    query: Option<string>,
    fragment: Option<string>,
};
