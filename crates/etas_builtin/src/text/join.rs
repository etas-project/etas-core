/// Failure to project an input or represent/allocate the complete output.
#[derive(Debug, PartialEq)]
pub enum JoinError<E> {
    Projection(E),
    OutputTooLarge,
}

/// Joins a repeatable borrowed projection without a temporary parts array.
/// The first traversal validates every part and sizes the output; the second
/// writes it. Cloning the iterator must repeat the same immutable inputs.
pub fn join_projected<'a, E, I>(parts: I, separator: &str) -> Result<String, JoinError<E>>
where
    I: Iterator<Item = Result<&'a str, E>> + Clone,
{
    let mut bytes = 0usize;
    let mut first = true;
    for part in parts.clone() {
        let part = part.map_err(JoinError::Projection)?;
        bytes = joined_size(bytes, part.len(), if first { 0 } else { separator.len() })
            .ok_or(JoinError::OutputTooLarge)?;
        first = false;
    }
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| JoinError::OutputTooLarge)?;
    for (index, part) in parts.enumerate() {
        let part = part.map_err(JoinError::Projection)?;
        if index != 0 {
            output.push_str(separator);
        }
        output.push_str(part);
    }
    Ok(output)
}

fn joined_size(bytes: usize, part: usize, separator: usize) -> Option<usize> {
    bytes.checked_add(separator)?.checked_add(part)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn joins_empty_unicode_and_empty_parts() {
        for parts in [
            vec![],
            vec![""],
            vec!["", "a", ""],
            vec!["中", "😀", "e\u{301}"],
        ] {
            for separator in ["", ":", "中😀"] {
                let actual =
                    join_projected(parts.iter().copied().map(Ok::<_, ()>), separator).unwrap();
                assert_eq!(actual, parts.join(separator));
            }
        }
    }

    #[test]
    fn rejects_projection_failure_and_size_overflow() {
        assert_eq!(
            join_projected([Ok("good"), Err("invalid")].into_iter(), ","),
            Err(JoinError::Projection("invalid"))
        );
        assert_eq!(joined_size(usize::MAX, 1, 0), None);
        assert_eq!(joined_size(usize::MAX, 0, 1), None);
        assert_eq!(joined_size(usize::MAX - 1, 1, 0), Some(usize::MAX));
    }
}
