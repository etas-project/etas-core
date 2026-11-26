use super::*;

pub(super) fn to_proto(contract: &ConstructorContract) -> ProtoConstructorContract {
    use proto_constructor_generic_argument::Value;
    ProtoConstructorContract {
        path: contract.path.clone(),
        generic_params: contract
            .generic_params
            .iter()
            .map(generic_param_to_proto)
            .collect(),
        agent: contract.agent,
        fields: contract
            .fields
            .iter()
            .map(|field| ProtoConstructorField {
                public: field.public,
                name: field.name.clone(),
                ty: Some(type_to_proto(&field.ty)),
                injection: field
                    .injection
                    .as_ref()
                    .map(|injection| ProtoConstructorInjection {
                        action: Some(effect_ref_to_proto(&injection.action)),
                        generic_args: injection
                            .generic_args
                            .iter()
                            .map(|arg| ProtoConstructorGenericArgument {
                                value: Some(match arg {
                                    ConstructorGenericArgument::Type(ty) => {
                                        Value::Type(type_to_proto(ty))
                                    }
                                    ConstructorGenericArgument::EffectRow(row) => {
                                        Value::EffectRow(effect_row_to_proto(row))
                                    }
                                    ConstructorGenericArgument::Infer => Value::Infer(true),
                                }),
                            })
                            .collect(),
                    }),
            })
            .collect(),
    }
}

pub(super) fn from_proto(
    contract: ProtoConstructorContract,
) -> Result<ConstructorContract, MetadataArtifactError> {
    use proto_constructor_generic_argument::Value;
    Ok(ConstructorContract {
        path: required_path(contract.path, "constructor declaration path")?,
        generic_params: contract
            .generic_params
            .into_iter()
            .map(generic_param_from_proto)
            .collect::<Result<_, _>>()?,
        agent: contract.agent,
        fields: contract
            .fields
            .into_iter()
            .map(|field| {
                if field.name.is_empty() {
                    return Err(invalid("constructor field name is required"));
                }
                Ok(ConstructorField {
                    public: field.public,
                    name: field.name,
                    ty: type_from_proto(
                        field
                            .ty
                            .ok_or_else(|| invalid("constructor field type is required"))?,
                    )?,
                    injection: field
                        .injection
                        .map(|injection| {
                            Ok::<_, MetadataArtifactError>(ConstructorInjection {
                                action: effect_ref_from_proto(injection.action.ok_or_else(
                                    || invalid("constructor provider action is required"),
                                )?)?,
                                generic_args: injection
                                    .generic_args
                                    .into_iter()
                                    .map(|arg| {
                                        Ok(match arg.value {
                                            Some(Value::Type(ty)) => {
                                                ConstructorGenericArgument::Type(type_from_proto(
                                                    ty,
                                                )?)
                                            }
                                            Some(Value::EffectRow(row)) => {
                                                ConstructorGenericArgument::EffectRow(
                                                    effect_row_from_proto(row)?,
                                                )
                                            }
                                            Some(Value::Infer(true)) => {
                                                ConstructorGenericArgument::Infer
                                            }
                                            _ => {
                                                return Err(invalid(
                                                    "constructor generic argument is required",
                                                ));
                                            }
                                        })
                                    })
                                    .collect::<Result<_, MetadataArtifactError>>()?,
                            })
                        })
                        .transpose()?,
                })
            })
            .collect::<Result<_, MetadataArtifactError>>()?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constructor_contract_roundtrip_preserves_declaration_order_and_binders() {
        let parameter = Type {
            kind: TypeKind::Var,
            name: "T".into(),
            ..Default::default()
        };
        let contract = ConstructorContract {
            path: vec!["library".into(), "Config".into()],
            generic_params: vec![GenericParam {
                requires_copy: None,
                name: "T".into(),
                ..Default::default()
            }],
            agent: false,
            fields: vec![
                ConstructorField {
                    public: true,
                    name: "z_first".into(),
                    ty: parameter.clone(),
                    injection: Some(ConstructorInjection {
                        action: EffectRef {
                            path: vec!["library".into(), "Supply".into(), "value".into()],
                            args: vec![EffectArg {
                                kind: EffectArgKind::Type,
                                ty: Some(parameter.clone()),
                                ..Default::default()
                            }],
                        },
                        generic_args: vec![
                            ConstructorGenericArgument::Type(parameter.clone()),
                            ConstructorGenericArgument::EffectRow(EffectRow {
                                effects: Vec::new(),
                                tail: Some("E".into()),
                            }),
                            ConstructorGenericArgument::Infer,
                        ],
                    }),
                },
                ConstructorField {
                    public: true,
                    name: "a_second".into(),
                    ty: parameter,
                    injection: None,
                },
            ],
        };
        let metadata = PublicMetadata {
            constructor_contracts: vec![contract.clone()],
            ..Default::default()
        };
        let bytes = public_metadata_to_proto(&metadata).encode_to_vec();
        let decoded = public_metadata_from_proto(
            ProtoPublicMetadataSection::decode(bytes.as_slice()).unwrap(),
        )
        .unwrap();
        assert_eq!(decoded, metadata);
        let mut relocated = contract;
        relocated.remap_paths(|_, path| {
            std::iter::once("dependency".into())
                .chain(path.iter().cloned())
                .collect()
        });
        assert_eq!(
            relocated
                .fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>(),
            ["z_first", "a_second"]
        );
        assert_eq!(
            relocated.fields[0].injection.as_ref().unwrap().action.path,
            ["dependency", "library", "Supply", "value"]
        );
        assert_eq!(relocated.fields[0].ty.kind, TypeKind::Var);
        assert_eq!(relocated.fields[0].ty.name, "T");
    }
}
