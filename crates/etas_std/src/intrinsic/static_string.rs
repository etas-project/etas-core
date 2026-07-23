use super::StdIntrinsicId;
use crate::intrinsic;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntrinsicStaticStringSemantics {
    StringTransform {
        argument: usize,
        transform: StaticStringTransform,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StaticStringTransform {
    Trim,
    Lowercase,
    Uppercase,
}

pub fn intrinsic_static_string_semantics(
    intrinsic_id: StdIntrinsicId,
) -> Option<IntrinsicStaticStringSemantics> {
    let transform = match intrinsic_id.0 {
        intrinsic::pure::TEXT_TRIM => StaticStringTransform::Trim,
        intrinsic::pure::TEXT_LOWERCASE => StaticStringTransform::Lowercase,
        intrinsic::pure::TEXT_UPPERCASE => StaticStringTransform::Uppercase,
        _ => return None,
    };
    Some(IntrinsicStaticStringSemantics::StringTransform {
        argument: 0,
        transform,
    })
}
