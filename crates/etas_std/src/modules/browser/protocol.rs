use crate::{
    FlowDecl, IntrinsicDescriptor, IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl,
    StdIntrinsicId, StdRegistryBuilder, StdSymbolKind, TypeDecl, TypeDeclKind, intrinsic,
};

pub fn register(builder: &mut StdRegistryBuilder) {
    let module = builder.module(
        &["std", "browser", "protocol"],
        "Browser protocol substrate declarations.",
    );
    for name in [
        "BrowserProfile",
        "BrowserSession",
        "BrowserMessage",
        "BrowserScreenshot",
        "BrowserError",
    ] {
        builder.symbol(
            module,
            name,
            StdSymbolKind::Type,
            StdDecl::Type(TypeDecl::generic(name, &[], TypeDeclKind::Support)),
            "Browser protocol substrate support type.",
        );
    }
    browser_flow(
        builder,
        module,
        "attach",
        &["BrowserProfile"],
        "BrowserSession",
        "Browser.attach[profile]",
        intrinsic::runtime::BROWSER_ATTACH,
        "Attach to an existing browser protocol session.",
    );
    browser_flow(
        builder,
        module,
        "create",
        &["BrowserProfile"],
        "BrowserSession",
        "Browser.attach[profile]",
        intrinsic::runtime::BROWSER_CREATE,
        "Create a new browser protocol session.",
    );
    browser_flow(
        builder,
        module,
        "send",
        &["BrowserSession", "BrowserMessage"],
        "unit",
        "Browser.send[session]",
        intrinsic::runtime::BROWSER_SEND,
        "Send a browser protocol message.",
    );
    browser_flow(
        builder,
        module,
        "recv",
        &["BrowserSession"],
        "BrowserMessage",
        "Browser.recv[session]",
        intrinsic::runtime::BROWSER_RECV,
        "Receive a browser protocol message.",
    );
    browser_flow(
        builder,
        module,
        "close",
        &["BrowserSession"],
        "unit",
        "Browser.close[session]",
        intrinsic::runtime::BROWSER_CLOSE,
        "Close a browser protocol session.",
    );
    browser_flow(
        builder,
        module,
        "screenshot",
        &["BrowserSession"],
        "BrowserScreenshot",
        "Browser.screenshot[session]",
        intrinsic::runtime::BROWSER_SCREENSHOT,
        "Capture a screenshot payload for a browser protocol session.",
    );
}

fn browser_flow(
    builder: &mut StdRegistryBuilder,
    module: crate::StdModuleId,
    name: &str,
    params: &[&str],
    output: &str,
    action: &str,
    id: u32,
    summary: &str,
) {
    builder.symbol_with_intrinsic(
        module,
        name,
        StdSymbolKind::Flow,
        StdDecl::Flow(FlowDecl::with_actions(
            name,
            params,
            output,
            &["Error[BrowserError]"],
            &[action],
        )),
        summary,
        Some(IntrinsicDescriptor {
            id: StdIntrinsicId(id),
            qualified_path: vec![
                "std".into(),
                "browser".into(),
                "protocol".into(),
                name.into(),
            ],
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
            latent_effect: crate::IntrinsicLatentEffect::None,
            memory_access: crate::IntrinsicMemoryAccess::None,
            runtime_requirement: crate::IntrinsicRuntimeRequirement::None,
        }),
    );
}
