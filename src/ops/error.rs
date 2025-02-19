//! Exit errors returned from an op.

use clap;

/// Non-zero exit status from an op.
///
/// Based in part on the execline convention
/// (see <https://skarnet.org/software/execline/exitcodes.html>).
///
/// All these commands exit
/// - 1 if they encounter an expected error
/// - 100 if they encounter a permanent error – “the user is holding it wrong”
/// - 101 if they encounter a programming error, like a panic or failed assert
/// - 111 if they encounter a temporary error, such as resource exhaustion
/// - 126 if there is a problem with the environment in which lorri is run
/// - 127 if they're trying to execute into a program and cannot find it

#[derive(Debug)]
pub struct ExitError {
    /// Exit code of the process, should be non-zero
    exitcode: i32,
    /// The error
    error: anyhow::Error,
}

impl ExitError {
    /// Exit 1 to signify a generic expected error
    /// (e.g. something that sometimes just goes wrong, like a nix build).
    pub fn expected_error<E>(err: E) -> ExitError
    where
        E: Into<anyhow::Error>,
    {
        ExitError {
            exitcode: 1,
            error: err.into(),
        }
    }

    /// Exit 100 to signify a user error (“the user is holding it wrong”).
    /// This is a permanent error, if the program is executed the same way
    /// it should crash with 100 again.
    pub fn user_error<E>(err: E) -> ExitError
    where
        E: Into<anyhow::Error>,
    {
        ExitError {
            exitcode: 100,
            error: err.into(),
        }
    }

    /// Exit 101 to signify an unexpected crash (failing assertion or panic).
    /// This is the same exit code that `panic!()` emits.
    pub fn panic<E>(err: E) -> ExitError
    where
        E: Into<anyhow::Error>,
    {
        ExitError {
            exitcode: 101,
            error: err.into(),
        }
    }

    /// Exit 111 to signify a temporary error (such as resource exhaustion)
    pub fn temporary<E>(err: E) -> ExitError
    where
        E: Into<anyhow::Error>,
    {
        ExitError {
            exitcode: 111,
            error: err.into(),
        }
    }

    /// Exit 126 to signify an environment problem
    /// (the user has set up stuff incorrectly so lorri cannot work)
    pub fn environment_problem<E>(err: E) -> ExitError
    where
        E: Into<anyhow::Error>,
    {
        ExitError {
            exitcode: 126,
            error: err.into(),
        }
    }

    /// Exit 127 to signify a missing executable.
    pub fn missing_executable<E>(err: E) -> ExitError
    where
        E: Into<anyhow::Error>,
    {
        ExitError {
            exitcode: 127,
            error: err.into(),
        }
    }

    /// Exit code of the failure message, guaranteed to be > 0
    pub fn exitcode(&self) -> i32 {
        self.exitcode
    }

    /// Exit message to be displayed to the user on stderr
    pub fn message(&self) -> String {
        // use the alternative form, since it includes the error source.
        // TODO: format with {:?} if -v was enabled (|| RUST_BACKTRACE?)
        format!("{:#}", &self.error)
    }
}

/// We count plain IO errors as temporary errors.
impl From<std::io::Error> for ExitError {
    fn from(e: std::io::Error) -> ExitError {
        ExitError::temporary(anyhow::anyhow!(e))
    }
}

impl From<clap::error::Error> for ExitError {
    fn from(err: clap::error::Error) -> Self {
        ExitError::user_error(err)
    }
}

/// enum that lists all the possible `ExitError`s we support.
/// See `ExitAs` for the use.
#[allow(missing_docs)]
pub enum ExitErrorType {
    ExpectedError,
    UserError,
    Panic,
    Temporary,
    EnvironmentProblem,
    MissingExecutable,
}

/// Helper trait to implement the kind of exit code an error variant would cause.
pub trait ExitAs {
    /// The `ExitErrorType` the implementing error should be converted to if it happens.
    fn exit_as(&self) -> ExitErrorType;
}

// TODO: the anyhow context is a trait not a type wrapping an error,
// so I don’t see how we can automatically impl From for the context & ExitAs.

/// For every error type which implements `ExitAs` and `Error` we can automatically convert them to an `ExitError`.
impl<Err> From<Err> for ExitError
where
    Err: Sync + Send + 'static,
    Err: std::error::Error + ExitAs,
{
    fn from(e: Err) -> ExitError {
        let exit_as = e.exit_as();
        use ExitErrorType::*;
        match exit_as {
            ExpectedError => ExitError::expected_error(e),
            UserError => ExitError::user_error(e),
            Panic => ExitError::panic(e),
            Temporary => ExitError::temporary(e),
            EnvironmentProblem => ExitError::environment_problem(e),
            MissingExecutable => ExitError::missing_executable(e),
        }
    }
}
