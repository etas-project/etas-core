use crate::{
    IntrinsicDispatch, IntrinsicPurity, LoweringHint, StdDecl, StdRegistryBuilder, StdSymbolKind,
    TypeDecl, TypeDeclKind, intrinsic,
};

use crate::modules::registration::{IntrinsicFlowRegistration, register_intrinsic_flow};

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
    register_intrinsic_flow(
        builder,
        module,
        &["std", "browser", "protocol"],
        IntrinsicFlowRegistration {
            name: "attach",
            type_params: &[],
            params: &["BrowserProfile"],
            output: "BrowserSession",
            public_effects: &["Error[BrowserError]"],
            requested_actions: &["Browser.attach[profile]"],
            intrinsic_id: intrinsic::runtime::BROWSER_ATTACH,
            summary: "Attach to an existing browser protocol session.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "browser", "protocol"],
        IntrinsicFlowRegistration {
            name: "create",
            type_params: &[],
            params: &["BrowserProfile"],
            output: "BrowserSession",
            public_effects: &["Error[BrowserError]"],
            requested_actions: &["Browser.attach[profile]"],
            intrinsic_id: intrinsic::runtime::BROWSER_CREATE,
            summary: "Create a new browser protocol session.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "browser", "protocol"],
        IntrinsicFlowRegistration {
            name: "send",
            type_params: &[],
            params: &["BrowserSession", "BrowserMessage"],
            output: "unit",
            public_effects: &["Error[BrowserError]"],
            requested_actions: &["Browser.send[session]"],
            intrinsic_id: intrinsic::runtime::BROWSER_SEND,
            summary: "Send a browser protocol message.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "browser", "protocol"],
        IntrinsicFlowRegistration {
            name: "recv",
            type_params: &[],
            params: &["BrowserSession"],
            output: "BrowserMessage",
            public_effects: &["Error[BrowserError]"],
            requested_actions: &["Browser.recv[session]"],
            intrinsic_id: intrinsic::runtime::BROWSER_RECV,
            summary: "Receive a browser protocol message.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "browser", "protocol"],
        IntrinsicFlowRegistration {
            name: "close",
            type_params: &[],
            params: &["BrowserSession"],
            output: "unit",
            public_effects: &["Error[BrowserError]"],
            requested_actions: &["Browser.close[session]"],
            intrinsic_id: intrinsic::runtime::BROWSER_CLOSE,
            summary: "Close a browser protocol session.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
    register_intrinsic_flow(
        builder,
        module,
        &["std", "browser", "protocol"],
        IntrinsicFlowRegistration {
            name: "screenshot",
            type_params: &[],
            params: &["BrowserSession"],
            output: "BrowserScreenshot",
            public_effects: &["Error[BrowserError]"],
            requested_actions: &["Browser.screenshot[session]"],
            intrinsic_id: intrinsic::runtime::BROWSER_SCREENSHOT,
            summary: "Capture a screenshot payload for a browser protocol session.",
            purity: IntrinsicPurity::Host,
            dispatch: IntrinsicDispatch::Host,
            lowering: LoweringHint::RuntimeCall,
        },
    );
}
