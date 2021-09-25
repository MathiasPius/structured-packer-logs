use std::{collections::HashMap, convert::TryFrom, str::Split};

use crate::{error::Error, event::{Artifact, Build, Event, EventKind, UI}};

/// TODO: Replace error type with actual error.
impl TryFrom<PartialArtifactLog> for Artifact {
    type Error = String;

    fn try_from(partial: PartialArtifactLog) -> Result<Self, Self::Error> {
        if let PartialArtifactLog::Done(artifact) = partial {
            Ok(artifact)
        } else {
            Err("artifact not done yet".into())
        }
    }
}

#[derive(Debug, Clone)]
enum PartialArtifactLog {
    Root,
    BuilderId {
        builder_id: String,
    },
    Id {
        builder_id: String,
        id: Option<String>,
    },
    String {
        builder_id: String,
        id: Option<String>,
        string: String,
    },
    ListingFiles {
        builder_id: String,
        id: Option<String>,
        string: String,
        count: usize,
        files: Vec<Option<String>>,
    },
    Done(Artifact),
}

impl Default for PartialArtifactLog {
    fn default() -> Self {
        PartialArtifactLog::Root
    }
}

fn expect_message(structure: &'static str, expected: &'static str, message: &str) {
    if message != expected {
        panic!(
            "unexpected message {} in {}, expected '{}'",
            message, structure, expected
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decoding {
    Partial,
    Done,
}

trait Decodeable {
    type Error;
    type Unit;
    fn try_decode<F: Fn(Self::Unit)>(
        &mut self,
        input: Split<&str>,
        callback: F,
    ) -> Result<Decoding, Self::Error>;
}

impl Decodeable for PartialArtifactLog {
    type Error = Error;
    type Unit = Artifact;

    fn try_decode<F: Fn(Self::Unit)>(
        &mut self,
        mut input: Split<&str>,
        callback: F,
    ) -> Result<Decoding, Self::Error> {
        let message = input
            .next()
            .expect("no message passed to partial artifact build logger");

        // Temporarily replace the contents of "self" with an empty PartialArtifactLog::Root
        // take the old content of self and mutate it, then replace the *self reference with
        // the mutated version. This is basically a funky way of doing a mutate-in-place for
        // a mutable reference to an enum.
        *self = match std::mem::replace(self, PartialArtifactLog::Root) {
            PartialArtifactLog::Root => {
                expect_message("partial artifact", "builder-id", message);

                let id = input.next().expect("no builder id specified");
                PartialArtifactLog::BuilderId {
                    builder_id: id.to_string(),
                }
            }
            PartialArtifactLog::BuilderId { builder_id } => {
                expect_message("partial artifact", "id", message);

                let id = match input.next().expect("no id specified") {
                    "" => None,
                    s => Some(s.to_string()),
                };

                PartialArtifactLog::Id { builder_id, id }
            }
            PartialArtifactLog::Id { builder_id, id } => {
                expect_message("partial artifact", "string", message);

                let string = input.next().expect("no builder id specified").to_string();
                PartialArtifactLog::String {
                    builder_id,
                    id,
                    string,
                }
            }
            PartialArtifactLog::String {
                builder_id,
                id,
                string,
            } => {
                expect_message("partial artifact", "files-count", message);
                let count: usize = input
                    .next()
                    .expect("no file count specified")
                    .parse()
                    .unwrap();

                PartialArtifactLog::ListingFiles {
                    builder_id,
                    id,
                    string,
                    count,
                    files: vec![None; count],
                }
            }
            PartialArtifactLog::ListingFiles {
                count,
                builder_id,
                id,
                string,
                mut files,
            } => {
                if count == 0 {
                    expect_message("partial artifact", "end", message);

                    let artifact = Artifact {
                        builder_id,
                        id,
                        files: files.into_iter().map(Option::unwrap).collect(),
                    };

                    callback(artifact.clone());
                    PartialArtifactLog::Done(artifact)
                } else {
                    expect_message("partial artifact", "file", message);

                    let file_id: usize =
                        input.next().expect("no file id specified").parse().unwrap();
                    let file_name = input.next().expect("no file name specified");

                    files[file_id].replace(file_name.to_string());

                    PartialArtifactLog::ListingFiles {
                        builder_id,
                        id,
                        string,
                        count: count - 1,
                        files,
                    }
                }
            }
            PartialArtifactLog::Done(_) => panic!("already finished the artifact"),
        };

        Ok(if let PartialArtifactLog::Done(_) = self {
            Decoding::Done
        } else {
            Decoding::Partial
        })
    }
}

#[derive(Debug, Clone)]
enum PartialBuildLog {
    Root,
    ListingArtifacts {
        count: usize,
        artifacts: Vec<Option<PartialArtifactLog>>,
    },
    Done(Build),
}

enum BuildLogEventKind {
    Artifact(Artifact),
    Done(Build),
}

impl Decodeable for PartialBuildLog {
    type Error = String;
    type Unit = BuildLogEventKind;

    fn try_decode<F: Fn(Self::Unit)>(
        &mut self,
        mut input: Split<&str>,
        callback: F,
    ) -> Result<Decoding, Self::Error> {
        let message = input
            .next()
            .expect("no message passed to partial artifact build logger");

        *self = match std::mem::replace(self, PartialBuildLog::Root) {
            PartialBuildLog::Root => {
                expect_message("partial build", "artifact-count", message);
                let count: usize = input
                    .next()
                    .expect("no artifact count specified")
                    .parse()
                    .unwrap();

                PartialBuildLog::ListingArtifacts {
                    count,
                    artifacts: vec![None; count],
                }
            }
            PartialBuildLog::ListingArtifacts {
                mut count,
                mut artifacts,
            } => {
                expect_message("partial build", "artifact", message);

                let artifact_id: usize = input
                    .next()
                    .expect("no artifact id specified")
                    .parse()
                    .unwrap();

                let decoded_artifact = artifacts[artifact_id]
                    .get_or_insert(PartialArtifactLog::Root)
                    .try_decode(input, |artifact| {
                        callback(BuildLogEventKind::Artifact(artifact))
                    })
                    .unwrap();

                // Only ever decrement the counter when we're 100% finished decoding
                // an artifact. This way we can keep track of whether a Build is done,
                // by checking if any un-decoded artifacts lay ahead.
                if decoded_artifact == Decoding::Done {
                    count -= 1;
                };

                if count == 0 {
                    let build = Build {
                        artifacts: artifacts
                            .into_iter()
                            .map(Option::unwrap)
                            .map(Artifact::try_from)
                            .map(Result::unwrap)
                            .collect(),
                    };

                    callback(BuildLogEventKind::Done(build.clone()));

                    PartialBuildLog::Done(build)
                } else {
                    PartialBuildLog::ListingArtifacts { count, artifacts }
                }
            }
            PartialBuildLog::Done(_) => todo!(),
        };

        Ok(if let PartialBuildLog::Done(_) = self {
            Decoding::Done
        } else {
            Decoding::Partial
        })
    }
}

#[derive(Default)]
pub struct EventLog {
    builds: HashMap<String, PartialBuildLog>,
}

impl Decodeable for EventLog {
    type Error = String;
    type Unit = Event;
    fn try_decode<F: Fn(Self::Unit)>(
        &mut self,
        mut input: Split<&str>,
        callback: F,
    ) -> Result<Decoding, Self::Error> {
        let timestamp = input.next().expect("no timestamp found").to_string();
        let build_name = input.next().expect("no build name found");

        // If this isn't tied to build_name, then it's global.
        if build_name.is_empty() {
            let message_type = input.next().expect("no message type in line");
            let kind = match message_type {
                "ui" => {
                    let ui_type = input.next().expect("no sub-type for ui event found");
                    EventKind::UI(match ui_type {
                        "say" => UI::Say(input.next().unwrap().to_string()),
                        "message" => UI::Message(input.next().unwrap().to_string()),
                        "error" => UI::Error(input.next().unwrap().to_string()),
                        _ => panic!("unexpected global ui message: {}", ui_type),
                    })
                }
                _ => panic!("unexpected global message_type: {}", message_type),
            };

            callback(Event { timestamp, kind });
        } else {
            let enrich = |build_event| {
                callback(Event {
                    // We have to clone the timestamp here to make this function Fn and not FnOnce,
                    // because the function could be called multiple times from the following
                    // Decodeable::try_decode call.
                    timestamp: timestamp.clone(),
                    kind: match build_event {
                        BuildLogEventKind::Artifact(artifact) => EventKind::Artifact {
                            build_name: build_name.to_string(),
                            artifact,
                        },
                        BuildLogEventKind::Done(build) => EventKind::Build { build },
                    },
                })
            };

            let log = self
                .builds
                .entry(build_name.to_string())
                .or_insert(PartialBuildLog::Root);

            log.try_decode(input, enrich).unwrap();
        }

        Ok(Decoding::Partial)
    }
}

#[cfg(test)]
mod tests {
    use super::EventLog;
    use crate::log::Decodeable;

    #[test]
    pub fn parse_build_log() {
        let raw_log = include_str!("../tests/example-build.log");

        let mut log = EventLog::default();
        for line in raw_log.lines() {
            log.try_decode(line.split(","), |event| {
                println!("{:#?}", event);
            })
            .unwrap();
        }
    }
}
