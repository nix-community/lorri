//! The daemon's RPC server.

use super::IndicateActivity;
use super::LoopHandlerEvent;
use crate::build_loop;
use crate::error;
use crate::internal_proto;
use crate::ops::error::ExitError;
use crate::proto;
use crate::socket::{BindLock, SocketPath};
use crate::NixFile;

use crossbeam_channel as chan;
use slog_scope::debug;
use std::convert::{TryFrom, TryInto};
use std::path::PathBuf;

/// The daemon server.
pub struct Server {
    activity_tx: chan::Sender<IndicateActivity>,
    build_tx: chan::Sender<LoopHandlerEvent>,
    socket_path: SocketPath,
    _lock: BindLock,
}

impl Server {
    /// Create a new Server. Locks the Unix socket path, so there can be only one Server instance
    /// per socket path at any time.
    pub fn new(
        socket_path: SocketPath,
        activity_tx: chan::Sender<IndicateActivity>,
        build_tx: chan::Sender<LoopHandlerEvent>,
    ) -> Result<Server, ExitError> {
        let lock = socket_path.lock()?;
        Ok(Server {
            socket_path,
            activity_tx,
            build_tx,
            _lock: lock,
        })
    }

    /// Serve the daemon endpoint.
    pub fn serve(self) -> Result<(), ExitError> {
        let address = &self.socket_path.address();
        let service = varlink::VarlinkService::new(
            /* vendor */ "org.nixos",
            /* product */ "lorri",
            /* version */ "0.1",
            /* url */ "https://github.com/nix-community/lorri",
            vec![Box::new(internal_proto::new(Box::new(self)))],
        );
        let initial_worker_threads = 1;
        let max_worker_threads = 10;
        let idle_timeout = 0;
        varlink::listen(
            service,
            address,
            initial_worker_threads,
            max_worker_threads,
            idle_timeout,
        )
        .map_err(|e| ExitError::temporary(format!("{}", e)))
    }
}

/// The actual varlink server implementation. See org.nixos.lorri.varlink for the interface
/// specification.
impl internal_proto::VarlinkInterface for Server {
    fn watch_shell(
        &self,
        call: &mut dyn internal_proto::Call_WatchShell,
        shell_nix: internal_proto::ShellNix,
    ) -> varlink::Result<()> {
        let p = PathBuf::from(&shell_nix.path);
        if p.is_file() {
            self.activity_tx
                .send(IndicateActivity {
                    nix_file: NixFile::from(p),
                })
                .expect("failed to indicate activity via channel");
            call.reply()
        } else {
            call.reply_invalid_parameter(format!("{:?}", shell_nix))
        }
    }
}

// TODO: remove when switching to a protocol that can do [u8]
fn try_file_to_string(file: &std::path::Path) -> Result<String, String> {
    match file.to_str() {
        None => Err(format!("file {} is not a valid utf-8 string. Varlink does not support non-utf8 strings, so we cannot serialize this file name. TODO: link issue", file.display())),
        Some(s) => Ok(s.to_owned())
    }
}

// TODO: remove when switching to a protocol that can do [u8]
fn try_nix_file_to_string(file: &NixFile) -> Result<String, String> {
    try_file_to_string(file.as_path())
}

// TODO: remove when switchint to a protocol that can do [u8]
fn log_line_to_string(ll: &crate::error::LogLine) -> String {
    ll.0.to_string_lossy().into_owned()
}

impl proto::VarlinkInterface for Server {
    fn monitor(&self, call: &mut dyn proto::Call_Monitor) -> varlink::Result<()> {
        if !call.wants_more() {
            return call.reply_invalid_parameter("wants_more".to_string());
        }

        let (tx, rx) = chan::unbounded();
        self.build_tx
            .send(LoopHandlerEvent::NewListener(tx))
            .map_err(|_| varlink::error::ErrorKind::Server)?;

        call.set_continues(true);
        for event in rx {
            debug!("event for varlink"; "event" => ?&event);
            // TODO: destructure the owned event instead of a pointer to the event here
            match event.try_into() {
                Ok(ev) => call.reply(ev),
                Err(e) => call.reply_invalid_parameter(e.to_string()),
            }?;
        }
        Ok(())
    }
}

// TODO: replace all these TryFrom instances with one explicit transformation function.
// This should reduce the boilerplate considerably.

impl TryFrom<&build_loop::Event> for proto::Event {
    type Error = String;

    fn try_from(ev: &build_loop::Event) -> Result<Self, Self::Error> {
        use build_loop::Event;
        use proto::Event_kind as kind;
        Ok(match ev {
            Event::SectionEnd => proto::Event {
                kind: kind::section_end,
                section: Some(proto::SectionMarker {}),
                reason: None,
                result: None,
                failure: None,
            },
            Event::Started { reason, .. } => proto::Event {
                kind: kind::started,
                section: None,
                reason: Some(reason.try_into()?),
                result: None,
                failure: None,
            },
            Event::Completed { .. } => proto::Event {
                kind: kind::completed,
                section: None,
                reason: None,
                result: Some(ev.try_into()?),
                failure: None,
            },
            Event::Failure { .. } => proto::Event {
                kind: kind::failure,
                section: None,
                reason: None,
                result: None,
                failure: Some(ev.try_into()?),
            },
        })
    }
}

impl TryFrom<build_loop::Event> for proto::Event {
    type Error = String;

    fn try_from(ev: build_loop::Event) -> Result<Self, Self::Error> {
        proto::Event::try_from(&ev)
    }
}

impl TryFrom<proto::Monitor_Reply> for build_loop::Event {
    type Error = String;

    fn try_from(mr: proto::Monitor_Reply) -> Result<Self, Self::Error> {
        build_loop::Event::try_from(mr.event)
    }
}

impl TryFrom<proto::Event> for build_loop::Event {
    type Error = String;

    fn try_from(re: proto::Event) -> Result<Self, Self::Error> {
        use proto::Event_kind::*;

        Ok(match re.kind {
            section_end => build_loop::Event::SectionEnd,
            started => re.reason.ok_or("missing reason")?.try_into()?,
            completed => re.result.ok_or("missing result")?.try_into()?,
            failure => re.failure.ok_or("missing failure log")?.try_into()?,
        })
    }
}

impl TryFrom<proto::Reason> for build_loop::Event {
    type Error = String;

    fn try_from(r: proto::Reason) -> Result<Self, Self::Error> {
        Ok(build_loop::Event::Started {
            nix_file: NixFile::from(r.project.clone().ok_or("missing nix file!")?),
            reason: r.try_into()?,
        })
    }
}

impl TryFrom<&build_loop::Reason> for proto::Reason {
    type Error = String;

    fn try_from(wr: &build_loop::Reason) -> Result<Self, Self::Error> {
        use build_loop::Reason;
        use proto::Reason_kind::*;

        Ok(match wr {
            Reason::PingReceived => proto::Reason {
                kind: ping_received,
                project: None,
                files: None,
            },
            Reason::ProjectAdded(nix_file) => proto::Reason {
                kind: project_added,
                project: Some(try_nix_file_to_string(nix_file)?),
                files: None,
            },
            Reason::FilesChanged(changed) => proto::Reason {
                kind: files_changed,
                project: None,
                files: Some(
                    changed
                        .iter()
                        .map(|pb| try_file_to_string(&pb))
                        .collect::<Result<Vec<String>, String>>()?,
                ),
            },
        })
    }
}

impl TryFrom<proto::Reason> for build_loop::Reason {
    type Error = String;

    fn try_from(rr: proto::Reason) -> Result<Self, Self::Error> {
        use build_loop::Reason;
        use proto::Reason_kind::*;

        Ok(match rr.kind {
            ping_received => Reason::PingReceived,
            project_added => {
                Reason::ProjectAdded(NixFile::from(rr.project.ok_or("missing nix file!")?))
            }
            files_changed => Reason::FilesChanged(
                rr.files
                    .ok_or("missing files!")?
                    .into_iter()
                    .map(PathBuf::from)
                    .collect(),
            ),
        })
    }
}

impl TryFrom<&build_loop::Event> for proto::Outcome {
    type Error = String;

    fn try_from(ev: &build_loop::Event) -> Result<Self, Self::Error> {
        if let build_loop::Event::Completed {
            nix_file,
            rooted_output_paths,
        } = ev
        {
            Ok(proto::Outcome {
                nix_file: try_nix_file_to_string(nix_file)?,
                project_root: rooted_output_paths.shell_gc_root.to_string(),
            })
        } else {
            Err(format!("can't make an Outcome out of {:?}", ev))
        }
    }
}

impl TryFrom<proto::Outcome> for build_loop::Event {
    type Error = String;

    fn try_from(o: proto::Outcome) -> Result<Self, Self::Error> {
        use crate::builder;
        use crate::project::roots;
        Ok(build_loop::Event::Completed {
            nix_file: NixFile::from(o.nix_file.clone()),
            rooted_output_paths: builder::OutputPaths {
                shell_gc_root: roots::RootPath(PathBuf::from(o.project_root)),
            },
        })
    }
}

impl TryFrom<&build_loop::Event> for proto::Failure {
    type Error = String;

    fn try_from(ev: &build_loop::Event) -> Result<Self, Self::Error> {
        use error::BuildError;
        use proto::Failure_kind::*;

        match ev {
            build_loop::Event::Failure { nix_file, failure } => Ok(match failure {
                BuildError::Io { msg } => proto::Failure {
                    kind: io,
                    nix_file: try_nix_file_to_string(nix_file)?,
                    io: Some(proto::IOFail {
                        message: msg.clone(),
                    }),
                    spawn: None,
                    exit: None,
                    output: None,
                },
                BuildError::Spawn { cmd, msg } => proto::Failure {
                    kind: spawn,
                    nix_file: try_nix_file_to_string(nix_file)?,
                    io: None,
                    spawn: Some(proto::SpawnFail {
                        command: cmd.clone(),
                        message: msg.clone(),
                    }),
                    exit: None,
                    output: None,
                },
                BuildError::Exit { cmd, status, logs } => proto::Failure {
                    kind: exit,
                    nix_file: try_nix_file_to_string(nix_file)?,
                    io: None,
                    spawn: None,
                    exit: Some(proto::ExitFail {
                        command: cmd.clone(),
                        status: status.map(|i| i as i64),
                        logs: logs.iter().map(log_line_to_string).collect(),
                    }),
                    output: None,
                },
                BuildError::Output { msg } => proto::Failure {
                    kind: output,
                    nix_file: try_nix_file_to_string(nix_file)?,
                    io: None,
                    spawn: None,
                    exit: None,
                    output: Some(proto::OutputFail {
                        message: msg.clone(),
                    }),
                },
            }),
            _ => Err(String::from("expecting build_loop::Event::Failure")),
        }
    }
}

impl TryFrom<proto::Failure> for build_loop::Event {
    type Error = String;

    fn try_from(f: proto::Failure) -> Result<Self, Self::Error> {
        Ok(build_loop::Event::Failure {
            nix_file: NixFile::from(f.nix_file.clone()),
            failure: f.try_into()?,
        })
    }
}

impl TryFrom<proto::Failure> for error::BuildError {
    type Error = &'static str;

    fn try_from(pf: proto::Failure) -> Result<Self, Self::Error> {
        use error::BuildError;
        use proto::Failure_kind::*;

        match pf {
            proto::Failure {
                kind: io,
                io: Some(proto::IOFail { message }),
                ..
            } => Ok(BuildError::Io { msg: message }),
            proto::Failure {
                kind: spawn,
                spawn: Some(proto::SpawnFail { command, message }),
                ..
            } => Ok(BuildError::Spawn {
                cmd: command,
                msg: message,
            }),
            proto::Failure {
                kind: exit,
                exit:
                    Some(proto::ExitFail {
                        command,
                        status,
                        logs,
                    }),
                ..
            } => Ok(BuildError::Exit {
                cmd: command,
                status: status.map(|i| i as i32),
                logs: logs.into_iter().map(error::LogLine::from).collect(),
            }),
            proto::Failure {
                kind: output,
                output: Some(proto::OutputFail { message }),
                ..
            } => Ok(BuildError::Output { msg: message }),
            _ => Err("unexpected form of proto::Failure"),
        }
    }
}

impl TryFrom<&NixFile> for internal_proto::ShellNix {
    type Error = &'static str;

    fn try_from(nix_file: &NixFile) -> Result<Self, Self::Error> {
        match nix_file.as_path().as_os_str().to_str() {
            Some(s) => Ok(internal_proto::ShellNix {
                path: s.to_string(),
            }),
            None => Err("nix file path is not UTF-8 clean"),
        }
    }
}

impl From<internal_proto::ShellNix> for NixFile {
    fn from(shell_nix: internal_proto::ShellNix) -> Self {
        Self::from(shell_nix.path)
    }
}
