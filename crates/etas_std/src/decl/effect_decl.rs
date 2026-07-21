use super::StdType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectDecl {
    pub name: String,
    pub params: Vec<String>,
    pub extends: Vec<String>,
    pub core: bool,
    pub stable_id: Option<u32>,
    pub runtime_requirement: Option<StdRuntimeRequirement>,
    pub high_impact_ack: bool,
}

impl EffectDecl {
    pub fn core(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            params: Vec::new(),
            extends: Vec::new(),
            core: true,
            stable_id: None,
            runtime_requirement: None,
            high_impact_ack: false,
        }
    }

    pub fn generic_core(name: &str, params: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            params: params.iter().map(|param| (*param).to_owned()).collect(),
            extends: Vec::new(),
            core: true,
            stable_id: None,
            runtime_requirement: None,
            high_impact_ack: false,
        }
    }

    pub fn standard(name: &str, extends: &[&str]) -> Self {
        Self {
            name: name.to_owned(),
            params: Vec::new(),
            extends: extends.iter().map(|effect| (*effect).to_owned()).collect(),
            core: false,
            stable_id: None,
            runtime_requirement: None,
            high_impact_ack: false,
        }
    }

    pub fn with_stable_id(mut self, stable_id: u32) -> Self {
        self.stable_id = Some(stable_id);
        self
    }

    pub fn with_runtime_requirement(mut self, requirement: StdRuntimeRequirement) -> Self {
        self.runtime_requirement = Some(requirement);
        self
    }

    pub fn with_high_impact_ack(mut self) -> Self {
        self.high_impact_ack = true;
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectActionDecl {
    pub owner: String,
    pub name: String,
    pub params: Vec<StdType>,
    pub effect_args: Vec<EffectActionArgKind>,
    pub output: StdType,
    pub stable_id: Option<u32>,
    pub runtime_requirement: Option<StdRuntimeRequirement>,
    pub high_impact_ack: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectActionArgKind {
    Type,
    MemoryPlace,
    StaticResourcePath { ty: &'static str },
    StringPattern,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StdRuntimeRequirement {
    Agentic,
    ToolCall,
    HostAuthority,
    DurableMemory,
    Approval,
    Checkpoint,
    Time,
    Network,
    Tcp,
    Stream,
    Tls,
    Browser,
    Console,
    FileIO,
    Command,
    SecretAccess,
    RuntimeHandler,
}

impl EffectActionDecl {
    pub fn new(owner: &str, name: &str, params: &[&str], output: &str) -> Self {
        Self {
            owner: owner.to_owned(),
            name: name.to_owned(),
            params: params.iter().map(|param| StdType::parse(param)).collect(),
            effect_args: Vec::new(),
            output: StdType::parse(output),
            stable_id: None,
            runtime_requirement: None,
            high_impact_ack: false,
        }
    }

    pub fn local(owner: &str, name: &str, params: &[&str], output: &str) -> Self {
        Self::new(owner, name, params, output)
    }

    pub fn with_effect_args(mut self, effect_args: &[EffectActionArgKind]) -> Self {
        self.effect_args = effect_args.to_vec();
        self
    }

    pub fn with_stable_id(mut self, stable_id: u32) -> Self {
        self.stable_id = Some(stable_id);
        self
    }

    pub fn with_runtime_requirement(mut self, requirement: StdRuntimeRequirement) -> Self {
        self.runtime_requirement = Some(requirement);
        self
    }

    pub fn with_high_impact_ack(mut self) -> Self {
        self.high_impact_ack = true;
        self
    }
}
