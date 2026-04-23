use crate::domain::{IdError, Space, SpaceId, TaskId};
use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ReferenceError {
    #[error(transparent)]
    InvalidId(#[from] IdError),
    #[error("task reference `{input}` is ambiguous")]
    AmbiguousTaskReference { input: String, matches: Vec<TaskId> },
    #[error("space reference `{input}` is ambiguous")]
    AmbiguousSpaceReference {
        input: String,
        matches: Vec<SpaceId>,
    },
    #[error("task reference `{0}` was not found")]
    TaskNotFound(String),
    #[error("space reference `{0}` was not found")]
    SpaceNotFound(String),
}

pub fn resolve_task_ref<'a, I>(reference: &str, ids: I) -> Result<TaskId, ReferenceError>
where
    I: IntoIterator<Item = &'a TaskId>,
{
    let ids = ids.into_iter().cloned().collect::<Vec<_>>();

    if let Some(exact) = ids.iter().find(|id| id.as_str() == reference) {
        return Ok(exact.clone());
    }

    let matches = ids
        .iter()
        .filter_map(|id| match id.matches_ref(reference) {
            Ok(true) => Some(Ok(id.clone())),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>()?;

    match matches.len() {
        0 => Err(ReferenceError::TaskNotFound(reference.to_owned())),
        1 => Ok(matches[0].clone()),
        _ => Err(ReferenceError::AmbiguousTaskReference {
            input: reference.to_owned(),
            matches,
        }),
    }
}

pub fn resolve_space_ref<'a, I>(reference: &str, spaces: I) -> Result<SpaceId, ReferenceError>
where
    I: IntoIterator<Item = &'a Space>,
{
    let spaces = spaces.into_iter().collect::<Vec<_>>();

    if let Some(exact) = spaces.iter().find(|space| space.id.as_str() == reference) {
        return Ok(exact.id.clone());
    }

    let id_matches = spaces
        .iter()
        .filter_map(|space| match space.id.matches_ref(reference) {
            Ok(true) => Some(Ok(space.id.clone())),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, _>>();

    match id_matches {
        Ok(matches) if matches.len() == 1 => return Ok(matches[0].clone()),
        Ok(matches) if matches.len() > 1 => {
            return Err(ReferenceError::AmbiguousSpaceReference {
                input: reference.to_owned(),
                matches,
            });
        }
        Ok(_) => {}
        Err(IdError::InvalidPrefix { .. }) | Err(IdError::InvalidReferenceLength { .. }) => {}
        Err(error) => return Err(error.into()),
    }

    let slug_matches = spaces
        .iter()
        .filter(|space| space.slug == reference)
        .map(|space| space.id.clone())
        .collect::<Vec<_>>();

    match slug_matches.len() {
        0 => Err(ReferenceError::SpaceNotFound(reference.to_owned())),
        1 => Ok(slug_matches[0].clone()),
        _ => Err(ReferenceError::AmbiguousSpaceReference {
            input: reference.to_owned(),
            matches: slug_matches,
        }),
    }
}
