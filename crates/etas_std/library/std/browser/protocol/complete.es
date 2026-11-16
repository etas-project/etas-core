module std.browser.protocol;

// Public declarations migrated from the legacy API inventory.
// Maintained as Etas source; native bindings retain existing execution during migration.

// Browser protocol substrate support type.
public type BrowserError;

// Browser protocol substrate support type.
public type BrowserMessage;

// Browser protocol substrate support type.
public type BrowserProfile;

// Browser protocol substrate support type.
public type BrowserScreenshot;

// Browser protocol substrate support type.
public type BrowserSession;

// Attach to an existing browser protocol session.
@std.binding("std.browser.protocol.attach")
public flow attach(arg0: std.browser.protocol.BrowserProfile) -> std.browser.protocol.BrowserSession ![Error<std.browser.protocol.BrowserError>] {}

// Close a browser protocol session.
@std.binding("std.browser.protocol.close")
public flow close(arg0: std.browser.protocol.BrowserSession) -> unit ![Error<std.browser.protocol.BrowserError>] {}

// Create a new browser protocol session.
@std.binding("std.browser.protocol.create")
public flow create(arg0: std.browser.protocol.BrowserProfile) -> std.browser.protocol.BrowserSession ![Error<std.browser.protocol.BrowserError>] {}

// Receive a browser protocol message.
@std.binding("std.browser.protocol.recv")
public flow recv(arg0: std.browser.protocol.BrowserSession) -> std.browser.protocol.BrowserMessage ![Error<std.browser.protocol.BrowserError>] {}

// Capture a screenshot payload for a browser protocol session.
@std.binding("std.browser.protocol.screenshot")
public flow screenshot(arg0: std.browser.protocol.BrowserSession) -> std.browser.protocol.BrowserScreenshot ![Error<std.browser.protocol.BrowserError>] {}

// Send a browser protocol message.
@std.binding("std.browser.protocol.send")
public flow send(arg0: std.browser.protocol.BrowserSession, arg1: std.browser.protocol.BrowserMessage) -> unit ![Error<std.browser.protocol.BrowserError>] {}
