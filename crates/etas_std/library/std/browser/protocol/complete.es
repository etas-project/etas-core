module std.browser.protocol;

// Browser protocol substrate support type.
public type BrowserError = { code: string, message: string };

// Browser protocol substrate support type.
public type BrowserMessage = std.json.JsonValue;

// Browser protocol substrate support type.
public type BrowserProfile = {
    name: string,
    endpoint: Option<std.host.url.Url>,
    headless: bool,
};

// Browser protocol substrate support type.
public type BrowserScreenshot = {
    content: bytes,
    mime_type: string,
    width: Option<usize>,
    height: Option<usize>,
};

// Browser protocol substrate support type.
type BrowserConnection;

public type BrowserSession = { private connection: BrowserConnection };

public flow attach(profile: BrowserProfile) -> BrowserSession ![Error<BrowserError>] {
    return perform std.effects.Browser.attach(profile);
}

public flow create(profile: BrowserProfile) -> BrowserSession ![Error<BrowserError>] {
    return perform std.effects.Browser.create(profile);
}

public flow close(session: BrowserSession) -> unit ![Error<BrowserError>] {
    perform std.effects.Browser.close(session);
}

public flow recv(session: BrowserSession) -> BrowserMessage ![Error<BrowserError>] {
    return perform std.effects.Browser.recv(session);
}

public flow screenshot(session: BrowserSession) -> BrowserScreenshot ![Error<BrowserError>] {
    return perform std.effects.Browser.screenshot(session);
}

public flow send(session: BrowserSession, message: BrowserMessage) -> unit ![Error<BrowserError>] {
    perform std.effects.Browser.send(session, message);
}
