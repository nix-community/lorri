use lorri::build_loop;
use lorri::cas::ContentAddressable;
use lorri::daemon::{Daemon, LoopHandlerEvent};
use lorri::nix::options::NixOptions;
use lorri::socket::path::SocketPath;
use lorri::AbsPathBuf;
use std::io::{Error, ErrorKind};
use std::thread;
use std::time::{Duration, Instant};

/// This tests the basic working of the client/daemon setup.
///
/// The daemon starts listening, the client sends a message
/// to request watching a nix file, the daemon starts a `build_loop`
/// and the test is successful once the `build_loop` signals
/// that the build is starting up (`Event::Started`).
#[test]
pub fn start_job_with_ping() -> std::io::Result<()> {
    // TODO: this code is a mess because Daeomon is not
    // nicely abstracted yet.
    //
    let tempdir = tempfile::tempdir()?;
    let shell_nix = tempdir.as_ref().join("shell.nix");
    std::fs::File::create(&shell_nix)?;

    let socket_path = SocketPath::from(
        AbsPathBuf::new(tempdir.path().to_owned())
            .unwrap()
            .join("socket"),
    );
    let address = socket_path.address();
    let cas =
        ContentAddressable::new(AbsPathBuf::new(tempdir.path().join("cas")).unwrap()).unwrap();
    let gc_root_dir = lorri::AbsPathBuf::new(tempdir.path().to_owned())
        .unwrap()
        .join("gc_root");

    // The daemon knows how to build stuff
    let (mut daemon, build_rx) = Daemon::new(NixOptions::empty());
    let accept_handle = thread::spawn(move || {
        daemon
            .serve(socket_path, &gc_root_dir, cas)
            .expect("failed to serve daemon endpoint");
    });

    connect(&address, Duration::from_millis(1000));

    lorri::ops::ping::main(
        lorri::NixFile::from(lorri::AbsPathBuf::new(shell_nix).unwrap()),
        Some(address),
    )
    .unwrap();

    // Read the first build event, which should be a `Started` message
    match build_rx.recv_timeout(Duration::from_millis(1000)).unwrap() {
        LoopHandlerEvent::BuildEvent(build_loop::Event::Started { .. }) => Ok(()),
        ev => Err(Error::new(
            ErrorKind::Other,
            format!("didn’t expect event {:?}", ev),
        )),
    }?;

    drop(accept_handle);
    drop(tempdir);
    Ok(())
}

/// The server side of the connection is started in a separate thread. This function waits until
/// the socket address is available for connection.
fn connect(
    address: &str,
    timeout: Duration,
) -> std::sync::Arc<std::sync::RwLock<varlink::Connection>> {
    let start = Instant::now();
    let mut connection = None;
    let mut last_error = None;
    while connection.is_none() {
        if start.elapsed() > timeout {
            panic!(
                "failed to connect to RPC endpoint within {:?}; last error: {:?}",
                timeout, last_error
            );
        }
        match varlink::Connection::with_address(&address) {
            Err(e) => last_error = Some(e),
            Ok(c) => connection = Some(c),
        }
    }
    connection.unwrap()
}
