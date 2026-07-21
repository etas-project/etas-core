#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AuthConfig {
    None,
    BearerToken(String),
    Header { name: String, value: String },
}

impl AuthConfig {
    pub fn headers(&self) -> Vec<(String, String)> {
        match self {
            Self::None => Vec::new(),
            Self::BearerToken(token) => {
                vec![("Authorization".to_owned(), format!("Bearer {token}"))]
            }
            Self::Header { name, value } => vec![(name.clone(), value.clone())],
        }
    }
}
