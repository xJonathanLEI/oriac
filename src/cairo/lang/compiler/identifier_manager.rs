use crate::cairo::lang::compiler::{
    identifier_definition::IdentifierDefinition, scoped_name::ScopedName,
};

use serde::Deserialize;
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[derive(Debug, thiserror::Error)]
pub enum IdentifierError {
    #[error(transparent)]
    MissingIdentifier(MissingIdentifierError),
    #[error(transparent)]
    NotAScope(NotAScopeError),
    #[error(transparent)]
    NotAnIdentifier(NotAnIdentifierError),
    #[error("cyclic aliasing detected")]
    CyclicAliasing,
}

#[derive(Debug, thiserror::Error)]
#[error("unknown identifier '{fullname}'.")]
pub struct MissingIdentifierError {
    pub fullname: ScopedName,
}

#[derive(Debug, thiserror::Error)]
#[error("identifier '{fullname}' is |definition.TYPE|, expected a scope.")]
pub struct NotAScopeError {
    fullname: ScopedName,
    definition: IdentifierDefinition,
    non_parsed: ScopedName,
}

#[derive(Debug, thiserror::Error)]
#[error("expected '{fullname}' to be an identifier, found a scope name.")]
pub struct NotAnIdentifierError {
    fullname: ScopedName,
}

pub struct IdentifierSearchResult {
    /// The definition of the searched identifier.
    pub identifier_definition: IdentifierDefinition,
    /// The canonical name of the identifier.
    pub canonical_name: ScopedName,
    /// The suffix of the name which was not parsed. For example, if one searches for 'x.y.z.w' and
    /// 'x.y' is a reference, then non_parsed will contain 'z.w'.
    pub non_parsed: ScopedName,
}

/// Manages the list of identifiers and their definitions.
#[derive(Debug)]
pub struct IdentifierManager {
    pub root: IdentifierScope,
    pub shared_state: Rc<RefCell<SharedState>>,
}

/// Represents a scope of identifiers.
#[derive(Debug, Default)]
pub struct IdentifierScope {
    pub shared_state: Rc<RefCell<SharedState>>,
    pub fullname: ScopedName,
    pub subscopes: HashMap<String, IdentifierScope>,
    pub identifiers: HashMap<String, IdentifierDefinition>,
}

#[derive(Debug, Default)]
pub struct SharedState {
    pub dict: HashMap<ScopedName, IdentifierDefinition>,
}

impl IdentifierManager {
    pub fn new() -> Self {
        let shared_state = Rc::new(RefCell::new(SharedState::default()));
        let root = IdentifierScope {
            shared_state: shared_state.clone(),
            fullname: ScopedName::default(),
            subscopes: HashMap::new(),
            identifiers: HashMap::new(),
        };

        Self { root, shared_state }
    }

    /// Adds an identifier with the given name and definition. Allows overriding an existing
    /// definition.
    pub fn add_identifier(&mut self, name: ScopedName, definition: IdentifierDefinition) {
        self.root.add_identifier(name, definition);
    }

    /// Finds the identifier with the given name. Includes alias resolution and a possibly
    /// non-parsed part.
    ///
    /// For example, if name='x.y.z', 'x' is an alias to 'a.b', and 'a.b.y' is a Reference
    /// definition, the function will return that reference with non_parsed='z'.
    pub fn get(&self, name: ScopedName) -> Result<IdentifierSearchResult, IdentifierError> {
        let mut current_identifier = name;

        // Use a set of visited identifiers to detect cycles.
        let mut visited_identifiers = HashSet::new();
        visited_identifiers.insert(current_identifier.clone());

        let mut result = self.root.get(current_identifier)?;

        // Resolve aliases.
        while let IdentifierDefinition::Alias { destination } = result.identifier_definition {
            current_identifier = &destination + &result.non_parsed;

            // Detect cycles.
            if visited_identifiers.contains(&current_identifier) {
                return Err(IdentifierError::CyclicAliasing);
            }
            visited_identifiers.insert(current_identifier.clone());

            result = self.root.get(current_identifier)?;
        }

        Ok(result)
    }

    /// Searches an identifier in the given accessible scopes. Later scopes override the first ones.
    pub fn search(
        &self,
        accessible_scopes: &[ScopedName],
        name: ScopedName,
    ) -> Result<IdentifierSearchResult, IdentifierError> {
        self._search_identifier(accessible_scopes, name)
    }

    /// Searches an identifier in the given accessible scopes. Later scopes override the first ones.
    fn _search_identifier(
        &self,
        accessible_scopes: &[ScopedName],
        name: ScopedName,
    ) -> Result<IdentifierSearchResult, IdentifierError> {
        // Later accessible scopes override the first ones.
        for scope in accessible_scopes.iter().rev() {
            match self.get(scope + &name) {
                Ok(result) => return Ok(result),
                Err(err) => match err {
                    IdentifierError::MissingIdentifier(exec) => {
                        // If the problem is already with the first item in name (or in the scope itself),
                        // just continue to the next accessible scope.
                        // For example, if there are two accessible scopes: 'scope0' and 'scope1', and both
                        // contain identifier named 'x'. If we are given 'x.y', we will only search for
                        // 'scope0.x.y', not 'scope1.x.y'.
                        // On the other hand if 'scope0' has no identifier 'x', we will look for
                        // 'scope1.x.y'.
                        if (scope + &name.slice(1..name.len())).startswith(&exec.fullname) {
                            continue;
                        }
                        return Err(IdentifierError::MissingIdentifier(exec));
                    }
                    _ => return Err(err),
                },
            };
        }

        Err(IdentifierError::MissingIdentifier(MissingIdentifierError {
            fullname: name.slice(1..name.len()),
        }))
    }
}
impl Default for IdentifierManager {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de> Deserialize<'de> for IdentifierManager {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let items: HashMap<ScopedName, IdentifierDefinition> = HashMap::deserialize(deserializer)?;

        let mut manager = Self::new();
        for (key, value) in items.iter() {
            manager.add_identifier(key.to_owned(), value.to_owned());
        }

        Ok(manager)
    }
}

impl IdentifierScope {
    /// Returns the direct child scope by name, or None if not present.
    pub fn get_single_scope(&self, name: &str) -> Option<&IdentifierScope> {
        self.subscopes.get(name)
    }

    /// Returns the direct child scope by name, or None if not present.
    pub fn get_single_scope_mut(&mut self, name: &str) -> Option<&mut IdentifierScope> {
        self.subscopes.get_mut(name)
    }

    /// Retrieves the identifer with the given name (possibly not fully parsed, without alias
    /// resolution).
    pub fn get(&self, name: ScopedName) -> Result<IdentifierSearchResult, IdentifierError> {
        if name.is_empty() {
            panic!("The 'name' argument must not be empty.");
        }

        let first_name = name.path[0].clone();
        let non_parsed = name.slice(1..name.path.len());
        let canonical_name = &self.fullname + first_name.clone();

        if name.len() > 1 {
            if let Some(scope) = self.get_single_scope(&first_name) {
                return scope.get(non_parsed);
            }
        }

        if let Some(identifier) = self.identifiers.get(&first_name) {
            return Ok(IdentifierSearchResult {
                identifier_definition: identifier.to_owned(),
                canonical_name,
                non_parsed,
            });
        }

        if self.subscopes.contains_key(&first_name) {
            return Err(IdentifierError::NotAnIdentifier(NotAnIdentifierError {
                fullname: &self.fullname + first_name,
            }));
        }

        Err(IdentifierError::MissingIdentifier(MissingIdentifierError {
            fullname: &self.fullname + first_name,
        }))
    }

    pub fn add_subscope(&mut self, first_name: String) {
        self.subscopes.insert(
            first_name.clone(),
            IdentifierScope {
                shared_state: self.shared_state.clone(),
                fullname: &self.fullname + first_name,
                subscopes: HashMap::new(),
                identifiers: HashMap::new(),
            },
        );
    }

    /// Adds an identifier to the manager. name is relative to the current scope.
    pub fn add_identifier(&mut self, name: ScopedName, definition: IdentifierDefinition) {
        if name.is_empty() {
            panic!("The name argument must not be empty.");
        }

        let first_name = name.path[0].clone();
        let non_parsed = name.slice(1..name.path.len());

        if name.path.len() == 1 {
            self.identifiers
                .insert(first_name.clone(), definition.clone());
            (*self.shared_state)
                .borrow_mut()
                .dict
                .insert(&self.fullname + first_name, definition);
            return;
        }

        let scope = match self.get_single_scope_mut(&first_name) {
            Some(scope) => scope,
            None => {
                self.add_subscope(first_name.clone());
                self.get_single_scope_mut(&first_name).unwrap()
            }
        };

        scope.add_identifier(non_parsed, definition);
    }
}
