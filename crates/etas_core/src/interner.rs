use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InternedString(pub u32);

#[derive(Clone, Debug, Default)]
pub struct StringInterner {
    strings: Vec<String>,
    ids: HashMap<String, InternedString>,
}

impl StringInterner {
    pub fn intern(&mut self, text: &str) -> InternedString {
        if let Some(id) = self.ids.get(text) {
            return *id;
        }

        let id = InternedString(self.strings.len().min(u32::MAX as usize) as u32);
        self.strings.push(text.to_owned());
        self.ids.insert(text.to_owned(), id);
        id
    }

    pub fn resolve(&self, id: InternedString) -> Option<&str> {
        self.strings.get(id.0 as usize).map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }
}
