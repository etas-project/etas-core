use etas_builtin::{BuiltinError, BuiltinValue, bytes::query::BytesQuery, call_pure_intrinsic};
use etas_std::{StdIntrinsicId, intrinsic::pure};

#[test]
fn sha256_known_answers_cover_padding_block_boundaries() {
    // Independent hashlib answers for bytes(i % 251 for i in 0..n).
    for (len, hex) in [
        (
            0,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            1,
            "6e340b9cffb37a989ca544e6bb780a2c78901d3fb33738768511a30617afa01d",
        ),
        (
            55,
            "463eb28e72f82e0a96c0a4cc53690c571281131f672aa229e0d45ae59b598b59",
        ),
        (
            56,
            "da2ae4d6b36748f2a318f23e7ab1dfdf45acdc9d049bd80e59de82a60895f562",
        ),
        (
            63,
            "29af2686fd53374a36b0846694cc342177e428d1647515f078784d69cdb9e488",
        ),
        (
            64,
            "fdeab9acf3710362bd2658cdc9a29e8f9c757fcf9811603a8c447cd1d9151108",
        ),
        (
            65,
            "4bfd2c8b6f1eec7a2afeb48b934ee4b2694182027e6d0fc075074f2fabb31781",
        ),
        (
            119,
            "da18797ed7c3a777f0847f429724a2d8cd5138e6ed2895c3fa1a6d39d18f7ec6",
        ),
        (
            120,
            "f52b23db1fbb6ded89ef42a23ce0c8922c45f25c50b568a93bf1c075420bbb7c",
        ),
        (
            127,
            "92ca0fa6651ee2f97b884b7246a562fa71250fedefe5ebf270d31c546bfea976",
        ),
        (
            128,
            "471fb943aa23c511f6f72f8d1652d9c880cfa392ad80503120547703e56a2be5",
        ),
        (
            129,
            "5099c6a56203f9687f7d33f4bfdf576d31dc91f6b695ecea38b2770c87631135",
        ),
        (
            1000,
            "4e4c294b331f7a2099a379bec34b9f9fc03dc46ab465d998f4d683da53487e6d",
        ),
    ] {
        let input: Vec<u8> = (0..len).map(|i| (i % 251) as u8).collect();
        let expected = BuiltinValue::Bytes(
            (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect(),
        );
        assert_eq!(
            BytesQuery::Sha256.evaluate(&[&input]).unwrap(),
            expected,
            "len={len}"
        );
        assert_eq!(
            call_pure_intrinsic(
                StdIntrinsicId(pure::CRYPTO_SHA256),
                &[BuiltinValue::Bytes(input)]
            )
            .unwrap(),
            expected
        );
    }
}

#[test]
fn borrowed_byte_queries_match_dispatch_and_reject_wrong_arity() {
    for (id, args) in [
        (pure::BYTES_LEN, vec![vec![0, 255]]),
        (pure::CRYPTO_SHA256, vec![vec![1, 2, 3]]),
        (pure::CRYPTO_CONSTANT_TIME_EQ, vec![vec![], vec![]]),
        (pure::CRYPTO_CONSTANT_TIME_EQ, vec![vec![0], vec![]]),
        (pure::CRYPTO_CONSTANT_TIME_EQ, vec![vec![1, 2], vec![1, 3]]),
        (pure::CRYPTO_CONSTANT_TIME_EQ, vec![vec![1, 2], vec![3, 2]]),
        (
            pure::CRYPTO_CONSTANT_TIME_EQ,
            vec![vec![0, 255], vec![0, 255]],
        ),
    ] {
        let query = BytesQuery::for_intrinsic(StdIntrinsicId(id)).unwrap();
        let borrowed: Vec<_> = args.iter().map(Vec::as_slice).collect();
        let owned: Vec<_> = args.iter().cloned().map(BuiltinValue::Bytes).collect();
        assert_eq!(
            query.evaluate(&borrowed),
            call_pure_intrinsic(StdIntrinsicId(id), &owned)
        );
        if id == pure::CRYPTO_CONSTANT_TIME_EQ {
            assert_eq!(
                query.evaluate(&borrowed).unwrap(),
                BuiltinValue::Bool(args[0] == args[1])
            );
        }
        assert!(matches!(
            query.evaluate(&[]),
            Err(BuiltinError::ArityMismatch { .. })
        ));
        assert!(matches!(
            query.evaluate(&[&[], &[], &[]]),
            Err(BuiltinError::ArityMismatch { .. })
        ));
    }
    assert!(BytesQuery::for_intrinsic(StdIntrinsicId(pure::TEXT_LEN)).is_none());
}
