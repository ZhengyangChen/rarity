use thiserror::Error;

#[derive(Error, Debug)]
pub enum GraphError {
    #[error("node name {0} is reserved, use another name please")]
    ReservedName(String),
    #[error("node name {0} is already in use, use another name please")]
    RepeatedName(String),
    #[error("node name {0} not found")]
    UnknownName(String),
    #[error("link error")]
    LinkError(#[from] LinkError),
    #[error("tap error")]
    TapError(#[from] TapError),
}

#[derive(Error, Debug)]
pub enum LinkError {
    #[error("node name {0} not found")]
    UnknownName(String),
    #[error("link source {0} already linked")]
    LinkedSource(String),
    #[error("link target {0} already linked")]
    LinkedTarget(String),
    #[error("{0} is not a link source")]
    InvalidLinkSource(String),
    #[error("{0} is not a link target")]
    InvalidLinkTarget(String),
    #[error("{0} {1} are already tapped")]
    LinkIsTapped(String, String),
}

#[derive(Error, Debug)]
pub enum TapError {
    #[error("node name {0} not found")]
    UnknownName(String),
    #[error("{0} already tapped")]
    TappedTarget(String),
    #[error("{0} is not a tap target")]
    InvalidTapTarget(String),
    #[error("{0} {1} are already linked")]
    TapIsLinked(String, String),
}

pub type GraphResult<T> = Result<T, GraphError>;
pub type LinkResult<T> = Result<T, LinkError>;
pub type TapResult<T> = Result<T, TapError>;
