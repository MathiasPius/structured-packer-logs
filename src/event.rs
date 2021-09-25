#[derive(Debug, Clone)]
pub enum UI {
    Say(String),
    Message(String),
    Error(String),
}

#[derive(Debug, Clone)]
pub struct Event {
    pub(crate) timestamp: String,
    pub(crate) kind: EventKind,
}

#[derive(Debug, Clone)]
pub enum EventKind {
    UI(UI),
    Artifact {
        build_name: String,
        artifact: Artifact,
    },
    Build {
        build: Build,
    },
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub builder_id: String,
    pub id: Option<String>,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Build {
    pub artifacts: Vec<Artifact>,
}
