pub trait Idx: Copy {
    fn from_u32(value: u32) -> Self;
    fn into_usize(self) -> usize;
}

#[macro_export]
macro_rules! id_type {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub u32);

        impl $name {
            pub fn index(self) -> usize {
                self.0 as usize
            }
        }

        impl $crate::Idx for $name {
            fn from_u32(value: u32) -> Self {
                Self(value)
            }

            fn into_usize(self) -> usize {
                self.index()
            }
        }

        impl From<u32> for $name {
            fn from(value: u32) -> Self {
                Self(value)
            }
        }

        impl From<$name> for usize {
            fn from(value: $name) -> Self {
                value.index()
            }
        }

        impl $crate::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: $crate::serde::Serializer,
            {
                serializer.serialize_u32(self.0)
            }
        }

        impl<'de> $crate::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: $crate::serde::Deserializer<'de>,
            {
                Ok(Self(<u32 as $crate::serde::Deserialize>::deserialize(
                    deserializer,
                )?))
            }
        }
    };
}
