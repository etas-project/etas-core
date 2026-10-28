use super::*;

pub(super) fn request_template_to_proto(
    template: &RequestInstantiationTemplate,
) -> ProtoRequestInstantiationTemplate {
    ProtoRequestInstantiationTemplate {
        actions: template
            .actions
            .iter()
            .map(|instance| ProtoActionRequestTemplate {
                action: Some(effect_ref_to_proto(&instance.action)),
                type_bindings: instance.type_bindings.as_ref().map(|bindings| {
                    ProtoActionTypeBindings {
                        bindings: bindings
                            .iter()
                            .map(|binding| ProtoTypeField {
                                name: binding.parameter.clone(),
                                ty: Some(type_to_proto(&binding.ty)),
                            })
                            .collect(),
                    }
                }),
            })
            .collect(),
        parameters: template
            .parameters
            .iter()
            .map(|parameter| ProtoParameterRequestTemplate {
                parameter: parameter.parameter.clone(),
                excluded: parameter.excluded.iter().map(effect_ref_to_proto).collect(),
            })
            .collect(),
    }
}

pub(super) fn request_template_from_proto(
    template: ProtoRequestInstantiationTemplate,
) -> Result<RequestInstantiationTemplate, MetadataArtifactError> {
    let template = RequestInstantiationTemplate {
        actions: template
            .actions
            .into_iter()
            .map(|instance| {
                Ok(ActionRequestTemplate {
                    action: effect_ref_from_proto(
                        instance
                            .action
                            .ok_or_else(|| invalid("request instance has no action identity"))?,
                    )?,
                    type_bindings: instance
                        .type_bindings
                        .map(|bindings| {
                            bindings
                                .bindings
                                .into_iter()
                                .map(|binding| {
                                    Ok(ActionTypeBinding {
                                        parameter: binding.name,
                                        ty: type_from_proto(binding.ty.ok_or_else(|| {
                                            invalid("request type binding has no type")
                                        })?)?,
                                    })
                                })
                                .collect::<Result<_, MetadataArtifactError>>()
                        })
                        .transpose()?,
                })
            })
            .collect::<Result<_, MetadataArtifactError>>()?,
        parameters: template
            .parameters
            .into_iter()
            .map(|parameter| {
                Ok(ParameterRequestTemplate {
                    parameter: parameter.parameter,
                    excluded: parameter
                        .excluded
                        .into_iter()
                        .map(effect_ref_from_proto)
                        .collect::<Result<_, _>>()?,
                })
            })
            .collect::<Result<_, MetadataArtifactError>>()?,
    };
    template.validate_shape()?;
    Ok(template)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action() -> EffectRef {
        EffectRef {
            path: vec!["Code".into(), "compile".into()],
            args: vec![],
        }
    }

    #[test]
    fn request_templates_round_trip_unknown_empty_and_generic_abis_distinctly() {
        for bindings in [
            None,
            Some(vec![]),
            Some(vec![ActionTypeBinding {
                parameter: "A".into(),
                ty: Type {
                    kind: TypeKind::Var,
                    name: "T".into(),
                    ..Default::default()
                },
            }]),
        ] {
            let template = RequestInstantiationTemplate {
                actions: vec![ActionRequestTemplate {
                    action: action(),
                    type_bindings: bindings,
                }],
                parameters: vec![ParameterRequestTemplate {
                    parameter: "callback".into(),
                    excluded: vec![action()],
                }],
            };
            let bytes = request_template_to_proto(&template).encode_to_vec();
            let decoded = request_template_from_proto(
                ProtoRequestInstantiationTemplate::decode(bytes.as_slice()).unwrap(),
            )
            .unwrap();
            assert_eq!(decoded, template);
        }
    }

    #[test]
    fn request_templates_reject_duplicate_binders_and_missing_action_identity() {
        let binding = ActionTypeBinding {
            parameter: "A".into(),
            ty: Type {
                kind: TypeKind::Var,
                name: "T".into(),
                ..Default::default()
            },
        };
        let template = RequestInstantiationTemplate {
            actions: vec![ActionRequestTemplate {
                action: action(),
                type_bindings: Some(vec![binding.clone(), binding]),
            }],
            parameters: vec![],
        };
        assert!(request_template_from_proto(request_template_to_proto(&template)).is_err());
        let wire = ProtoRequestInstantiationTemplate {
            actions: vec![ProtoActionRequestTemplate {
                action: None,
                type_bindings: None,
            }],
            parameters: vec![],
        };
        assert!(request_template_from_proto(wire).is_err());
    }

    #[test]
    fn request_templates_share_the_artifact_depth_and_node_budgets() {
        let mut ty = Type {
            kind: TypeKind::Primitive,
            name: "string".into(),
            ..Default::default()
        };
        for _ in 0..70 {
            ty = Type {
                kind: TypeKind::Option,
                children: vec![ty],
                ..Default::default()
            };
        }
        let mut metadata = PackageMetadata::default();
        metadata
            .public_metadata
            .effect_summaries
            .push(EffectSummary {
                item: vec!["requests".into(), "build".into()],
                handler_requests: Some(RequestInstantiationTemplate {
                    actions: vec![ActionRequestTemplate {
                        action: action(),
                        type_bindings: Some(vec![ActionTypeBinding {
                            parameter: "A".into(),
                            ty,
                        }]),
                    }],
                    parameters: vec![],
                }),
                ..Default::default()
            });
        let error = package_metadata_to_sections(&metadata).unwrap_err();
        assert!(error.to_string().contains("maximum depth"), "{error}");
        metadata.public_metadata.effect_summaries[0].handler_requests =
            Some(RequestInstantiationTemplate {
                actions: vec![
                    ActionRequestTemplate {
                        action: action(),
                        type_bindings: None
                    };
                    crate::MAX_METADATA_GRAPH_NODES
                ],
                parameters: vec![],
            });
        let error = package_metadata_to_sections(&metadata).unwrap_err();
        assert!(error.to_string().contains("maximum node count"), "{error}");
    }
}
