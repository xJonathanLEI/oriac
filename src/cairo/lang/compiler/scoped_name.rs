use serde::{de::Error as DeError, Deserialize, Serialize, Serializer};
use std::{fmt::Display, ops::Range, str::FromStr};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash)]
pub struct ScopedName {
    pub path: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("empty namespace is not supported")]
    EmptyNamespace,
}

impl ScopedName {
    pub const SEPARATOR: &'static str = ".";

    pub fn new(path: Vec<String>) -> Result<Self, Error> {
        for segment in path.iter() {
            if segment.is_empty() {
                return Err(Error::EmptyNamespace);
            }
        }
        Ok(Self { path })
    }

    pub fn len(&self) -> usize {
        self.path.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn slice(&self, range: Range<usize>) -> Self {
        Self {
            path: self.path[range]
                .iter()
                .map(|item| item.to_owned())
                .collect::<Vec<_>>(),
        }
    }

    pub fn startswith(&self, other: &ScopedName) -> bool {
        if self.len() < other.len() {
            false
        } else {
            self.path[0..other.len()] == other.path
        }
    }
}

impl std::ops::Add<String> for &ScopedName {
    type Output = ScopedName;

    fn add(self, rhs: String) -> Self::Output {
        let mut path = self.path.clone();
        path.push(rhs);

        ScopedName { path }
    }
}

impl std::ops::Add<&ScopedName> for &ScopedName {
    type Output = ScopedName;

    fn add(self, rhs: &ScopedName) -> Self::Output {
        let mut path = self.path.clone();
        for segment in rhs.path.iter() {
            path.push(segment.to_owned());
        }

        ScopedName { path }
    }
}

impl Display for ScopedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.join(Self::SEPARATOR))
    }
}

impl FromStr for ScopedName {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(
            s.split(Self::SEPARATOR)
                .map(|item| item.to_owned())
                .collect::<Vec<_>>(),
        )
    }
}

impl Serialize for ScopedName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&format!("{}", self))
    }
}

impl<'de> Deserialize<'de> for ScopedName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        value
            .parse::<Self>()
            .map_err(|err| DeError::custom(format!("invalid scoped name string: {}", err)))
    }
}
