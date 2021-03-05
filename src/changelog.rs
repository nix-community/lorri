//! Data structures to interpret the lorri change log.

/// A representation of the release.nix change log format.
use vec1::{vec1, Vec1};

#[derive(Deserialize, Debug)]
pub struct Log {
    /// a list of ordered change log entries, newest first.
    pub entries: Vec<Entry>,
}

/// A specific changelog entry
#[derive(Deserialize, Debug)]
pub struct Entry {
    /// The version number (note: increasing number, not x.y.z)
    pub version: usize,

    /// A plain-text blob of change log text.
    pub changes: String,
}

trait ValidationFor<Err>: Sized
where
    Self: Sized,
{
    fn get_field(&self, field_name: &str) -> Result<&Self, Err>;
    fn get_string(&self) -> Result<&str, Err>;
}

struct Val<'a, In, Out, Err> {
    input: &'a In,
    path: Path,
    val: ValState<Out, Err>,
}

impl<'a, In, Err> Val<'a, In, (), Err> {
    fn empty(input: &'a In, path: Path) -> Self {
        Val {
            input,
            path,
            val: ValState::Ok(()),
        }
    }

    pub fn new(input: &'a In) -> Self {
        Val {
            input,
            path: Path(vec![]),
            val: ValState::Ok(()),
        }
    }
}

enum ValState<Out, Err> {
    Init,
    Ok(Out),
    Err(Vec1<ValError<Err>>),
}

#[derive(Clone, Debug)]
struct ValError<Err> {
    path: Path,
    err: Err,
}

#[derive(Clone, Debug)]
struct Path(Vec<String>);

impl Path {
    fn join(&self, right: String) -> Self {
        match self {
            Path(vec) => {
                let mut p = vec.clone();
                p.push(right);
                Path(p)
            }
        }
    }
}

impl<'a, Err, In, Out> Val<'a, In, Out, Err>
where
    In: ValidationFor<Err>,
    Err: Clone,
{
    pub fn string(self) -> Val<'a, In, &'a str, Err> {
        let path = self.path.clone();
        match self.input.get_string() {
            Ok(string) => self.ok(string),
            Err(err) => self.set_val(ValState::Err(vec1![ValError { path, err }])),
        }
    }

    pub fn get<Out2, F>(self, field_name: &str, to_val: F) -> Val<'a, In, (Out, Out2), Err>
    where
        F: Fn(Val<'a, In, (), Err>) -> Val<'a, In, Out2, Err>,
    {
        let path = self.path.join(field_name.to_string());
        match self.input.get_field(field_name) {
            Ok(inner) => self.merge(to_val(Val::empty(&inner, path))),
            Err(err) => self.set_val(ValState::Err(vec1![ValError { path, err }])),
        }
    }

    pub fn validate(self) -> Result<Out, Vec1<ValError<Err>>> {
        match self.val {
            ValState::Init => panic!("no parsing was done"),
            ValState::Ok(out) => Ok(out),
            ValState::Err(err) => Err(err),
        }
    }

    fn set_val<NewOut>(self, val: ValState<NewOut, Err>) -> Val<'a, In, NewOut, Err> {
        Val {
            input: self.input,
            path: self.path,
            val: val,
        }
    }

    fn drain(&mut self) -> ValState<Out, Err> {
        std::mem::replace(&mut self.val, ValState::Init)
    }

    fn ok<T>(mut self, a: T) -> Val<'a, In, T, Err> {
        let selfval = self.drain();
        let val = match selfval {
            ValState::Init => panic!("tmp"),
            ValState::Ok(ok) => ValState::Ok(a),
            ValState::Err(err) => ValState::Err(err),
        };
        self.set_val(val)
    }

    fn merge<Out2>(mut self, mut other: Val<'a, In, Out2, Err>) -> Val<'a, In, (Out, Out2), Err> {
        let selfval = self.drain();
        let otherval = other.drain();
        let val = match (selfval, otherval) {
            (ValState::Init, _) => panic!("don’t call merge with the init state"),
            (_, ValState::Init) => panic!("don’t call merge with the init state"),
            (ValState::Ok(ok), ValState::Ok(ok2)) => ValState::Ok((ok, ok2)),
            (ValState::Ok(_), ValState::Err(err)) => ValState::Err(err),
            (ValState::Err(err), ValState::Ok(_)) => ValState::Err(err),
            (ValState::Err(err), ValState::Err(err2)) => ValState::Err({
                let mut new = err.clone();
                new.extend_from_slice(&err2[..]);
                new
            }),
        };
        self.set_val(val)
    }
}

// struct Fields<'a, T>(Val<'a, T>);

// impl<'a, Err, In> Fields<'a, In>
//     where In: ValidationFor<Err>
// {
//     fn get<U>(self) -> Self<(T, U)> {
//         match self {
//             Fields(v) => v
//                 }
//     }

// }

#[derive(Clone, Debug)]
enum Error {
    NotATable { was: &'static str },
    NotAString { was: &'static str },
    FieldDoesNotExist { name: String },
}

impl ValidationFor<Error> for toml::Value {
    fn get_field(&self, field_name: &str) -> Result<&Self, Error> {
        use toml::Value;
        match self {
            Value::Table(map) => map.get(field_name).ok_or(Error::FieldDoesNotExist {
                name: field_name.to_string(),
            }),
            was => Err(Error::NotATable {
                was: was.type_str(),
            }),
        }
    }

    fn get_string(&self) -> Result<&str, Error> {
        use toml::Value;
        match self {
            Value::String(string) => Ok(string),
            was => Err(Error::NotAString {
                was: was.type_str(),
            }),
        }
    }
}

// impl Validate<toml::Value, Error> for Log {
//     use toml::Value;

//     fn val(v: Val) -> Val<Entry, Error> {
//         v.fields(
//             |f|
//             let (version, (changes, ())) =
//                 f.get("version").get("changes");
//             Entry {version, changes};
//         )
//     }
// }

fn parse_changelog(v: &toml::Value) -> Result<((), &str), Vec1<ValError<Error>>> {
    let mut val = Val::new(v);
    val.get("version", |v| v.string()).validate()
}

#[test]
fn test_changelog() {
    assert_eq!(
        Val::new(&toml::toml!(bla = "bar"))
            .get("version", |v| v.string())
            .get("foo", |v| v.string())
            .validate()
            .unwrap(),
        (((), "version"), "foo")
    )
}
