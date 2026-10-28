use serde::{Serialize, Serializer, ser::Error};

use super::DescriptorDetail;
use crate::descriptor::{
    DescriptorError, DescriptorGraph, DescriptorNode, EffectRowInfoRef, SpecInfo, TraceInfoRef,
    TypeInfoRef,
};

impl Serialize for DescriptorDetail {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        enum Export {
            Type {
                graph: DescriptorGraph,
                ty: TypeInfoRef,
            },
            EffectRow {
                graph: DescriptorGraph,
                row: EffectRowInfoRef,
            },
            Spec {
                graph: DescriptorGraph,
                spec: SpecInfo,
            },
            Trace {
                graph: DescriptorGraph,
                trace: TraceInfoRef,
            },
        }
        let projected = match self {
            Self::Type { graph, ty } => Export::Type {
                graph: graph
                    .project(DescriptorNode::Type(*ty))
                    .map_err(S::Error::custom)?,
                ty: TypeInfoRef(0),
            },
            Self::EffectRow { graph, row } => Export::EffectRow {
                graph: graph
                    .project(DescriptorNode::Row(*row))
                    .map_err(S::Error::custom)?,
                row: EffectRowInfoRef(0),
            },
            Self::Trace { graph, trace } => Export::Trace {
                graph: graph
                    .project(DescriptorNode::Trace(*trace))
                    .map_err(S::Error::custom)?,
                trace: TraceInfoRef(0),
            },
            Self::Spec { graph, spec } => {
                let roots = spec
                    .arguments
                    .iter()
                    .map(|ty| DescriptorNode::Type(*ty))
                    .collect::<Vec<_>>();
                let (graph, roots) = graph.project_roots(&roots).map_err(S::Error::custom)?;
                let arguments = roots
                    .into_iter()
                    .map(|root| match root {
                        DescriptorNode::Type(ty) => Ok(ty),
                        node => Err(DescriptorError::Invalid {
                            node,
                            reason: "spec projection returned a non-type root",
                        }),
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(S::Error::custom)?;
                Export::Spec {
                    graph,
                    spec: SpecInfo {
                        definition: spec.definition.clone(),
                        arguments,
                    },
                }
            }
        };
        projected.serialize(serializer)
    }
}
