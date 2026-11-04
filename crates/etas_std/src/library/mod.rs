use crate::StdLibrarySourceId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StdLibrarySource {
    pub id: StdLibrarySourceId,
    pub virtual_path: &'static str,
    pub module_path: &'static [&'static str],
    pub source: &'static str,
    pub dependencies: &'static [StdLibrarySourceId],
    pub implementations: &'static [&'static str],
}

const TEXT_ASSEMBLY_IMPLEMENTATIONS: &[&str] = &[
    "concat",
    "join",
    "repeat",
    "pad_start",
    "pad_end",
    "reverse_chars",
];
const TEXT_FORMAT_IMPLEMENTATIONS: &[&str] = &["format"];

const SOURCES: &[StdLibrarySource] = &[
    StdLibrarySource {
        id: StdLibrarySourceId::TextAssembly,
        virtual_path: "etas-std://library/std/text/assembly.es",
        module_path: &["__etas_std", "text", "assembly"],
        source: include_str!("../../library/std/text/assembly.es"),
        dependencies: &[],
        implementations: TEXT_ASSEMBLY_IMPLEMENTATIONS,
    },
    StdLibrarySource {
        id: StdLibrarySourceId::TextFormat,
        virtual_path: "etas-std://library/std/text/format.es",
        module_path: &["__etas_std", "text", "format"],
        source: include_str!("../../library/std/text/format.es"),
        dependencies: &[],
        implementations: TEXT_FORMAT_IMPLEMENTATIONS,
    },
];

pub fn standard_library_sources() -> &'static [StdLibrarySource] {
    SOURCES
}

pub fn standard_library_source(id: StdLibrarySourceId) -> &'static StdLibrarySource {
    SOURCES
        .iter()
        .find(|source| source.id == id)
        .expect("every typed standard library source id has a bundled source asset")
}
