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
    join_text_projected(parts, separator)
}

/// A two-pass join without flattened input payloads. Chunk traversal can need
/// metadata; the final output owns its bytes and separator headers are reused.
pub fn join_text_projected<'a, T, E, I>(parts: I, separator: &T) -> Result<String, JoinError<E>>
where
    T: super::input::TextInput + ?Sized + 'a,
    I: Iterator<Item = Result<&'a T, E>> + Clone,
{
    let mut bytes = 0usize;
    let mut first = true;
    let mut separated = false;
    for part in parts.clone() {
        let part = part.map_err(JoinError::Projection)?;
        separated |= !first;
        bytes = joined_size(
            bytes,
            part.byte_len(),
            if first { 0 } else { separator.byte_len() },
        )
        .ok_or(JoinError::OutputTooLarge)?;
        first = false;
    }
    let mut output = String::new();
    output
        .try_reserve_exact(bytes)
        .map_err(|_| JoinError::OutputTooLarge)?;
    let separator_chunks = if !separated || separator.contiguous().is_some() {
        Vec::new()
    } else {
        separator.chunks().collect::<Vec<_>>()
    };
    for (index, part) in parts.enumerate() {
        let part = part.map_err(JoinError::Projection)?;
        if index != 0 {
            if let Some(separator) = separator.contiguous() {
                output.push_str(separator);
            } else {
                for chunk in &separator_chunks {
                    output.push_str(chunk);
                }
            }
        }
        for chunk in part.chunks() {
            output.push_str(chunk);
        }
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
