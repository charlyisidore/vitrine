//! Parse and manipulate URLs.
//!
//! This module provides the [`Url`] type for working with URLs.

use serde::{Deserialize, Serialize};

pub use self::path::Path as UrlPath;
use self::{authority::Authority, path::Path};

/// An owned and mutable URL.
#[derive(Clone, Debug, Default, Hash, PartialEq, Serialize, Deserialize)]
pub struct Url(String);

impl Url {
    /// Create an empty URL.
    pub fn new() -> Self {
        Self(String::new())
    }

    /// Create an empty URL with given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Self(String::with_capacity(capacity))
    }

    /// Return a reference to the inner [`str`] slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Return mutable reference to the inner [`str`] slice.
    pub fn as_mut_str(&mut self) -> &mut str {
        &mut self.0
    }

    /// Return a mutable reference to the inner [`String`].
    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.0
    }

    /// Consume the [`Url`] and return the inner [`String`].
    pub fn into_string(self) -> String {
        self.0
    }

    /// Return an iterator over the [`Component`]s of the URL.
    pub fn components(&self) -> Components {
        Components {
            url: self.0.trim(),
            state: State::Start,
        }
    }

    /// Return the scheme component, if any.
    pub fn scheme(&self) -> Option<&str> {
        self.components().find_map(|component| match component {
            Component::Scheme(s) => Some(s),
            _ => None,
        })
    }

    /// Return the path component.
    pub fn path(&self) -> &str {
        self.components()
            .find_map(|component| match component {
                Component::Path(s) => Some(s),
                _ => None,
            })
            .expect("must have path component")
    }

    /// Return the query component, if any.
    pub fn query(&self) -> Option<&str> {
        self.components().find_map(|component| match component {
            Component::Query(s) => Some(s),
            _ => None,
        })
    }

    /// Return the fragment component, if any.
    pub fn fragment(&self) -> Option<&str> {
        self.components().find_map(|component| match component {
            Component::Fragment(s) => Some(s),
            _ => None,
        })
    }

    /// Set the fragment component.
    pub fn set_fragment(&mut self, fragment: impl AsRef<str>) {
        let fragment = fragment.as_ref();
        if let Some(s) = self.fragment() {
            self.0.truncate(self.0.len() - s.len() - 1);
        }
        debug_assert!(self.0.find('#').is_none());
        if !fragment.is_empty() {
            self.0.reserve_exact(1 + fragment.len());
            self.0.push('#');
            self.0.push_str(fragment);
        }
    }

    /// Create a [`Url`] like [`self`], but with given fragment.
    pub fn with_fragment(&mut self, fragment: impl AsRef<str>) -> Self {
        let mut url = self.clone();
        url.set_fragment(fragment);
        url
    }

    /// Normalize the URL.
    pub fn normalize(&self) -> Self {
        let mut url = String::with_capacity(self.0.len());
        let mut scheme: Option<String> = None;
        let mut absolute = false;
        for component in self.components() {
            match component {
                Component::Scheme(s) => {
                    url.push_str(&s.to_ascii_lowercase());
                    url.push(':');
                    scheme = Some(s.to_ascii_lowercase());
                },
                Component::Authority(s) => {
                    url.push_str("//");
                    if let Some(scheme) = scheme.as_ref() {
                        let s = Authority::from(s).normalize(scheme);
                        url.push_str(s.as_str());
                    } else {
                        url.push_str(s);
                    }
                    absolute = true;
                },
                Component::Path(s) => {
                    // Normalize path if scheme `http`, `https`, or unspecified
                    if scheme
                        .as_ref()
                        .filter(|scheme| !["http", "https"].contains(&scheme.as_str()))
                        .is_none()
                    {
                        let s = Path::from(s).normalize(absolute);
                        url.push_str(s.as_str());
                    } else {
                        url.push_str(s);
                    }
                },
                Component::Query(s) => {
                    if !s.is_empty() {
                        url.push('?');
                        url.push_str(s);
                    }
                },
                Component::Fragment(s) => {
                    if !s.is_empty() {
                        url.push('#');
                        url.push_str(s);
                    }
                },
            }
        }
        Self(url)
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Url {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<T> From<T> for Url
where
    T: Into<String>,
{
    fn from(value: T) -> Self {
        Self(value.into())
    }
}

/// An iterator over the [`Component`]s of a [`Url`].
#[derive(Debug)]
pub struct Components<'a> {
    url: &'a str,
    state: State,
}

/// A URL component.
#[derive(Debug, PartialEq)]
pub enum Component<'a> {
    /// A scheme.
    Scheme(&'a str),
    /// An authority.
    Authority(&'a str),
    /// A path.
    Path(&'a str),
    /// A query.
    Query(&'a str),
    /// A fragment.
    Fragment(&'a str),
}

/// URL parse state.
#[derive(Debug)]
enum State {
    /// Parsing the scheme, the authority, or the path component.
    Start,
    /// Parsing the authority component.
    Authority,
    /// Parsing the path component.
    Path,
    /// Parsing the query component.
    Query,
    /// Parsing the fragment component.
    Fragment,
    /// Finished parsing.
    End,
}

impl<'a> Components<'a> {
    /// Parse the URL until the predicate returns true.
    ///
    /// Returns a triplet `(position, char, component_str)`.
    fn parse_until(&self, predicate: impl Fn(&usize, &char) -> bool) -> (usize, char, &'a str) {
        self.url
            .char_indices()
            .find(|(i, c)| (predicate)(i, c))
            .map(|(i, c)| (i, c, &self.url[..i]))
            .unwrap_or((self.url.len(), '\0', self.url))
    }

    /// Parse the scheme, the authority, or the path component.
    fn parse_start(&mut self) -> Option<Component<'a>> {
        let (i, c, component) = self.parse_until(|i, c| match i {
            0 => !c.is_ascii_alphabetic(),
            _ => !c.is_ascii_alphanumeric() && !['+', '-', '.'].contains(c),
        });

        match c {
            ':' => {
                self.url = &self.url[i + 1..];
                if self.url.starts_with("//") {
                    self.url = &self.url[2..];
                    self.state = State::Authority;
                } else {
                    self.state = State::Path;
                }
                Some(Component::Scheme(component))
            },
            '/' if i == 0 && self.url.starts_with("//") => {
                self.url = &self.url[2..];
                self.parse_authority()
            },
            '\0' => {
                self.state = State::End;
                Some(Component::Path(self.url))
            },
            _ => self.parse_path(),
        }
    }

    /// Parse the authority component.
    fn parse_authority(&mut self) -> Option<Component<'a>> {
        let (i, c, component) = self.parse_until(|_, c| ['/', '?', '#'].contains(c));

        match c {
            '/' => {
                self.url = &self.url[i..];
                self.state = State::Path;
            },
            '?' => {
                self.url = &self.url[i + 1..];
                self.state = State::Query;
            },
            '#' => {
                self.url = &self.url[i + 1..];
                self.state = State::Fragment;
            },
            '\0' => {
                self.url = &self.url[self.url.len()..];
                self.state = State::Path;
            },
            _ => unreachable!(),
        }

        Some(Component::Authority(component))
    }

    /// Parse the path component.
    fn parse_path(&mut self) -> Option<Component<'a>> {
        let (i, c, component) = self.parse_until(|_, c| ['?', '#'].contains(c));

        match c {
            '?' => {
                self.url = &self.url[i + 1..];
                self.state = State::Query;
            },
            '#' => {
                self.url = &self.url[i + 1..];
                self.state = State::Fragment;
            },
            '\0' => self.state = State::End,
            _ => unreachable!(),
        }

        Some(Component::Path(component))
    }

    /// Parse the query component.
    fn parse_query(&mut self) -> Option<Component<'a>> {
        let (i, c, component) = self.parse_until(|_, c| *c == '#');

        match c {
            '#' => {
                self.url = &self.url[i + 1..];
                self.state = State::Fragment;
            },
            '\0' => self.state = State::End,
            _ => unreachable!(),
        }

        Some(Component::Query(component))
    }

    /// Parse the fragment component.
    fn parse_fragment(&mut self) -> Option<Component<'a>> {
        self.state = State::End;
        Some(Component::Fragment(self.url))
    }
}

impl<'a> Iterator for Components<'a> {
    type Item = Component<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::Start => self.parse_start(),
            State::Authority => self.parse_authority(),
            State::Path => self.parse_path(),
            State::Query => self.parse_query(),
            State::Fragment => self.parse_fragment(),
            State::End => None,
        }
    }
}

impl Component<'_> {
    /// Return the component as a string slice.
    pub fn as_str(&self) -> &str {
        match self {
            Self::Scheme(s) => s,
            Self::Authority(s) => s,
            Self::Path(s) => s,
            Self::Query(s) => s,
            Self::Fragment(s) => s,
        }
    }
}

/// Parse and manipulate URL authority components.
pub mod authority {
    /// An URL authority component.
    #[derive(Clone, Debug, Default, Hash, PartialEq)]
    pub struct Authority(String);

    impl Authority {
        /// Return a reference to the inner [`str`] slice.
        pub fn as_str(&self) -> &str {
            &self.0
        }

        /// Consume the [`Path`] and return the inner [`String`].
        pub fn into_string(self) -> String {
            self.0
        }

        /// Return an iterator over the [`Component`]s of the authority.
        pub fn components(&self) -> Components {
            Components {
                authority: &self.0,
                state: State::Start,
            }
        }

        /// Return the user info component, if any.
        pub fn userinfo(&self) -> Option<&str> {
            self.components().find_map(|component| match component {
                Component::Userinfo(s) => Some(s),
                _ => None,
            })
        }

        /// Return the host component.
        pub fn host(&self) -> &str {
            self.components()
                .find_map(|component| match component {
                    Component::Host(s) => Some(s),
                    _ => None,
                })
                .expect("must have host component")
        }

        /// Return the port component, if any.
        pub fn port(&self) -> Option<&str> {
            self.components().find_map(|component| match component {
                Component::Port(s) => Some(s),
                _ => None,
            })
        }

        /// Normalize the authority component.
        pub fn normalize(&self, scheme: &str) -> Self {
            let mut authority = String::with_capacity(self.0.len());
            for component in self.components() {
                match component {
                    Component::Userinfo(s) => {
                        authority.push_str(s);
                        authority.push('@');
                    },
                    Component::Host(s) => authority.push_str(&s.to_ascii_lowercase()),
                    Component::Port(s) => match (scheme, s) {
                        ("ftp", "21")
                        | ("http", "80")
                        | ("https", "443")
                        | ("ws", "80")
                        | ("wss", "443") => {},
                        _ => {
                            authority.push(':');
                            authority.push_str(s);
                        },
                    },
                }
            }
            Self(authority)
        }
    }

    impl<T> From<T> for Authority
    where
        T: Into<String>,
    {
        fn from(value: T) -> Self {
            Self(value.into())
        }
    }

    /// An iterator over the [`Component`]s of a [`Authority`].
    #[derive(Debug)]
    pub struct Components<'a> {
        authority: &'a str,
        state: State,
    }

    /// An URL path component.
    #[derive(Debug, PartialEq)]
    pub enum Component<'a> {
        /// User information component (before `@`).
        Userinfo(&'a str),
        /// Host component.
        Host(&'a str),
        /// Port component.
        Port(&'a str),
    }

    /// Authority parse state.
    #[derive(Debug)]
    enum State {
        Start,
        Host,
        Port,
        End,
    }

    impl<'a> Components<'a> {
        /// Parse the userinfo, or the host component.
        fn parse_start(&mut self) -> Option<Component<'a>> {
            if let Some(i) = self.authority.find('@') {
                let component = &self.authority[..i];
                self.authority = &self.authority[i + 1..];
                self.state = State::Host;
                Some(Component::Userinfo(component))
            } else {
                self.parse_host()
            }
        }

        /// Parse the host component.
        fn parse_host(&mut self) -> Option<Component<'a>> {
            let mut inside_brackets = false;
            if let Some(i) = self.authority.find(|c| match c {
                '[' => {
                    inside_brackets = true;
                    false
                },
                ']' => {
                    inside_brackets = false;
                    false
                },
                ':' => !inside_brackets,
                _ => false,
            }) {
                let component = &self.authority[..i];
                self.authority = &self.authority[i + 1..];
                self.state = State::Port;
                Some(Component::Host(component))
            } else {
                self.state = State::End;
                Some(Component::Host(self.authority))
            }
        }

        /// Parse the port component.
        fn parse_port(&mut self) -> Option<Component<'a>> {
            self.state = State::End;
            Some(Component::Port(self.authority))
        }
    }

    impl<'a> Iterator for Components<'a> {
        type Item = Component<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            match self.state {
                State::Start => self.parse_start(),
                State::Host => self.parse_host(),
                State::Port => self.parse_port(),
                State::End => None,
            }
        }
    }

    impl Component<'_> {
        /// Return the component as a string slice.
        pub fn as_str(&self) -> &str {
            match self {
                Self::Userinfo(s) => s,
                Self::Host(s) => s,
                Self::Port(s) => s,
            }
        }
    }
}

/// Parse and manipulate URL paths.
pub mod path {
    /// An URL path.
    #[derive(Clone, Debug, Default, Hash, PartialEq)]
    pub struct Path(String);

    impl Path {
        /// Create an empty path.
        pub fn new() -> Self {
            Self(String::new())
        }

        /// Create an empty path with given capacity.
        pub fn with_capacity(capacity: usize) -> Self {
            Self(String::with_capacity(capacity))
        }

        /// Return a reference to the inner [`str`] slice.
        pub fn as_str(&self) -> &str {
            &self.0
        }

        /// Consume the [`Path`] and return the inner [`String`].
        pub fn into_string(self) -> String {
            self.0
        }

        /// Check if the path is absolute.
        pub fn is_absolute(&self) -> bool {
            self.0.starts_with('/')
        }

        /// Check if the path is relative.
        pub fn is_relative(&self) -> bool {
            !self.is_absolute()
        }

        /// Return an iterator over the segments of the path.
        pub fn segments(&self) -> Segments {
            Segments { path: &self.0 }
        }

        /// Return an iterator over the [`Component`]s of the path.
        pub fn components(&self) -> Components {
            Components { path: &self.0 }
        }

        /// Return the file name, if any.
        pub fn file_name(&self) -> Option<&str> {
            self.components()
                .last()
                .and_then(|component| match component {
                    Component::Normal(s) => Some(s),
                    _ => None,
                })
        }

        /// Join a path to `self`.
        pub fn join(&self, path: impl AsRef<str>) -> Self {
            let path = path.as_ref();
            Self(if self.0.ends_with('/') {
                format!("{}{}", self.0, path)
            } else {
                format!("{}/{}", self.0, path)
            })
        }

        /// Normalize the path.
        pub fn normalize(&self, absolute: bool) -> Self {
            if self.0.is_empty() {
                return Self::from(if absolute {
                    Component::RootDir.as_str()
                } else {
                    Component::CurDir.as_str()
                });
            }

            let has_root = absolute || self.is_absolute();
            let mut result: Vec<Component> = Vec::new();

            for component in self.components() {
                match component {
                    Component::Normal(_) => result.push(component),
                    Component::ParentDir => {
                        if !has_root && result.is_empty()
                            || result
                                .last()
                                .map_or(false, |c| matches!(c, Component::ParentDir))
                        {
                            result.push(component);
                        } else {
                            result.pop();
                        }
                    },
                    Component::RootDir | Component::CurDir => {},
                }
            }

            let root = if has_root {
                Self::from(Component::RootDir.as_str())
            } else {
                Self::with_capacity(self.0.len())
            };

            let result = result.iter().enumerate().fold(root, |mut p, (i, c)| {
                p.0.push_str(c.as_str());
                if i + 1 < result.len() || !c.as_str().contains('.') {
                    p.0.push('/');
                }
                p
            });

            if result.as_str().is_empty() {
                return Self::from(Component::CurDir.as_str());
            }

            result
        }
    }

    impl std::fmt::Display for Path {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl<T> From<T> for Path
    where
        T: Into<String>,
    {
        fn from(value: T) -> Self {
            Self(value.into())
        }
    }

    /// An iterator over the segments of a [`Path`].
    #[derive(Debug)]
    pub struct Segments<'a> {
        path: &'a str,
    }

    impl<'a> Iterator for Segments<'a> {
        type Item = &'a str;

        fn next(&mut self) -> Option<Self::Item> {
            if self.path.is_empty() {
                return None;
            }
            if let Some(i) = self.path.find('/') {
                let segment = &self.path[..i + 1];
                self.path = self.path[i..].trim_start_matches('/');
                Some(segment)
            } else {
                let segment = self.path;
                self.path = &self.path[self.path.len()..];
                Some(segment)
            }
        }
    }

    /// An iterator over the [`Component`]s of a [`Path`].
    #[derive(Debug)]
    pub struct Components<'a> {
        path: &'a str,
    }

    /// An URL path component.
    #[derive(Debug, PartialEq)]
    pub enum Component<'a> {
        /// The root directory component (`/`).
        RootDir,
        /// A reference to the current directory (`.`).
        CurDir,
        /// A reference to the parent directory (`..`).
        ParentDir,
        /// A normal component.
        Normal(&'a str),
    }

    impl<'a> Iterator for Components<'a> {
        type Item = Component<'a>;

        fn next(&mut self) -> Option<Self::Item> {
            if self.path.is_empty() {
                return None;
            }
            if self.path.starts_with('/') {
                self.path = self.path.trim_start_matches('/');
                return Some(Component::RootDir);
            }
            let segment = match self.path.find('/') {
                Some(i) => {
                    let (segment, rest) = self.path.split_at(i);
                    self.path = rest.trim_start_matches('/');
                    segment
                },
                None => std::mem::take(&mut self.path),
            };
            match segment {
                "" => None,
                "." => Some(Component::CurDir),
                ".." => Some(Component::ParentDir),
                segment => Some(Component::Normal(segment)),
            }
        }
    }

    impl Component<'_> {
        /// Return the component as a string slice.
        pub fn as_str(&self) -> &str {
            match self {
                Self::RootDir => "/",
                Self::CurDir => ".",
                Self::ParentDir => "..",
                Self::Normal(s) => s,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{path::Path, Url};

    #[test]
    fn url_components() {
        use super::Component;

        const CASES: [(&str, &[Component]); 37] = [
            ("https://example.com", &[
                Component::Scheme("https"),
                Component::Authority("example.com"),
                Component::Path(""),
            ]),
            ("ftp://ftp.is.co.za/rfc/rfc1808.txt", &[
                Component::Scheme("ftp"),
                Component::Authority("ftp.is.co.za"),
                Component::Path("/rfc/rfc1808.txt"),
            ]),
            ("http://www.ietf.org/rfc/rfc2396.txt", &[
                Component::Scheme("http"),
                Component::Authority("www.ietf.org"),
                Component::Path("/rfc/rfc2396.txt"),
            ]),
            ("ldap://[2001:db8::7]/c=GB?objectClass?one", &[
                Component::Scheme("ldap"),
                Component::Authority("[2001:db8::7]"),
                Component::Path("/c=GB"),
                Component::Query("objectClass?one"),
            ]),
            ("mailto:John.Doe@example.com", &[
                Component::Scheme("mailto"),
                Component::Path("John.Doe@example.com"),
            ]),
            ("news:comp.infosystems.www.servers.unix", &[
                Component::Scheme("news"),
                Component::Path("comp.infosystems.www.servers.unix"),
            ]),
            ("tel:+1-816-555-1212", &[
                Component::Scheme("tel"),
                Component::Path("+1-816-555-1212"),
            ]),
            ("telnet://192.0.2.16:80/", &[
                Component::Scheme("telnet"),
                Component::Authority("192.0.2.16:80"),
                Component::Path("/"),
            ]),
            ("urn:oasis:names:specification:docbook:dtd:xml:4.1.2", &[
                Component::Scheme("urn"),
                Component::Path("oasis:names:specification:docbook:dtd:xml:4.1.2"),
            ]),
            ("foo://example.com:8042/over/there?name=ferret#nose", &[
                Component::Scheme("foo"),
                Component::Authority("example.com:8042"),
                Component::Path("/over/there"),
                Component::Query("name=ferret"),
                Component::Fragment("nose"),
            ]),
            ("urn:example:animal:ferret:nose", &[
                Component::Scheme("urn"),
                Component::Path("example:animal:ferret:nose"),
            ]),
            ("http://a/b/c/d;p?q", &[
                Component::Scheme("http"),
                Component::Authority("a"),
                Component::Path("/b/c/d;p"),
                Component::Query("q"),
            ]),
            ("data:,A%20brief%20note", &[
                Component::Scheme("data"),
                Component::Path(",A%20brief%20note"),
            ]),
            ("g:h", &[Component::Scheme("g"), Component::Path("h")]),
            ("g", &[Component::Path("g")]),
            ("./g", &[Component::Path("./g")]),
            ("g/", &[Component::Path("g/")]),
            ("/g", &[Component::Path("/g")]),
            ("//g", &[Component::Authority("g"), Component::Path("")]),
            ("?y", &[Component::Path(""), Component::Query("y")]),
            ("g?y", &[Component::Path("g"), Component::Query("y")]),
            ("#s", &[Component::Path(""), Component::Fragment("s")]),
            ("g#s", &[Component::Path("g"), Component::Fragment("s")]),
            ("g?y#s", &[
                Component::Path("g"),
                Component::Query("y"),
                Component::Fragment("s"),
            ]),
            (";x", &[Component::Path(";x")]),
            ("g;x", &[Component::Path("g;x")]),
            ("g;x?y#s", &[
                Component::Path("g;x"),
                Component::Query("y"),
                Component::Fragment("s"),
            ]),
            ("", &[Component::Path("")]),
            (".", &[Component::Path(".")]),
            ("./", &[Component::Path("./")]),
            ("..", &[Component::Path("..")]),
            ("../", &[Component::Path("../")]),
            ("../g", &[Component::Path("../g")]),
            ("../..", &[Component::Path("../..")]),
            ("../../", &[Component::Path("../../")]),
            ("../../g", &[Component::Path("../../g")]),
            ("//example.com", &[
                Component::Authority("example.com"),
                Component::Path(""),
            ]),
        ];

        for (input, expected) in CASES {
            let url = Url::from(input);
            let result: Vec<_> = url.components().collect();
            assert_eq!(result, expected, "{input:?}");
        }
    }

    #[test]
    fn authority_components() {
        use super::authority::{Authority, Component};

        const CASES: [(&str, &[Component]); 7] = [
            ("example.com", &[Component::Host("example.com")]),
            ("ftp.is.co.za", &[Component::Host("ftp.is.co.za")]),
            ("[2001:db8::7]", &[Component::Host("[2001:db8::7]")]),
            ("192.0.2.16:80", &[
                Component::Host("192.0.2.16"),
                Component::Port("80"),
            ]),
            ("example.com:8042", &[
                Component::Host("example.com"),
                Component::Port("8042"),
            ]),
            ("User@example.com", &[
                Component::Userinfo("User"),
                Component::Host("example.com"),
            ]),
            ("User@[2001:db8::7]:8042", &[
                Component::Userinfo("User"),
                Component::Host("[2001:db8::7]"),
                Component::Port("8042"),
            ]),
        ];

        for (input, expected) in CASES {
            let url = Authority::from(input);
            let result: Vec<_> = url.components().collect();
            assert_eq!(result, expected, "{input:?}");
        }
    }

    #[test]
    fn path_segments() {
        const CASES: [(&str, &[&str]); 20] = [
            ("", &[]),
            ("/", &["/"]),
            ("g", &["g"]),
            ("./g", &["./", "g"]),
            ("g/", &["g/"]),
            ("/g", &["/", "g"]),
            ("//g", &["/", "g"]),
            (";x", &[";x"]),
            ("g;x", &["g;x"]),
            (".", &["."]),
            ("./", &["./"]),
            ("..", &[".."]),
            ("../", &["../"]),
            ("../g", &["../", "g"]),
            ("../..", &["../", ".."]),
            ("../../", &["../", "../"]),
            ("../../g", &["../", "../", "g"]),
            ("/g/h", &["/", "g/", "h"]),
            ("/b/c/d;p", &["/", "b/", "c/", "d;p"]),
            ("/foo//bar", &["/", "foo/", "bar"]),
        ];

        for (input, expected) in CASES {
            let path = Path::from(input);
            let result: Vec<_> = path.segments().collect();
            assert_eq!(result, expected, "{input:?}");
        }
    }

    #[test]
    fn path_components() {
        use super::path::Component;

        const CASES: [(&str, &[Component]); 20] = [
            ("", &[]),
            ("/", &[Component::RootDir]),
            ("g", &[Component::Normal("g")]),
            ("./g", &[Component::CurDir, Component::Normal("g")]),
            ("g/", &[Component::Normal("g")]),
            ("/g", &[Component::RootDir, Component::Normal("g")]),
            ("//g", &[Component::RootDir, Component::Normal("g")]),
            (";x", &[Component::Normal(";x")]),
            ("g;x", &[Component::Normal("g;x")]),
            (".", &[Component::CurDir]),
            ("./", &[Component::CurDir]),
            ("..", &[Component::ParentDir]),
            ("../", &[Component::ParentDir]),
            ("../g", &[Component::ParentDir, Component::Normal("g")]),
            ("../..", &[Component::ParentDir, Component::ParentDir]),
            ("../../", &[Component::ParentDir, Component::ParentDir]),
            ("../../g", &[
                Component::ParentDir,
                Component::ParentDir,
                Component::Normal("g"),
            ]),
            ("/g/h", &[
                Component::RootDir,
                Component::Normal("g"),
                Component::Normal("h"),
            ]),
            ("/b/c/d;p", &[
                Component::RootDir,
                Component::Normal("b"),
                Component::Normal("c"),
                Component::Normal("d;p"),
            ]),
            ("/foo//bar", &[
                Component::RootDir,
                Component::Normal("foo"),
                Component::Normal("bar"),
            ]),
        ];

        for (input, expected) in CASES {
            let path = Path::from(input);
            let result: Vec<_> = path.components().collect();
            assert_eq!(result, expected, "{input:?}");
        }
    }

    #[test]
    fn normalize_url() {
        const CASES: [(&str, &str); 18] = [
            (
                "HTTPS://User@Example.COM/Foo",
                "https://User@example.com/Foo/",
            ),
            (
                "FTP://User@ftp.Example.COM:21/Foo",
                "ftp://User@ftp.example.com/Foo",
            ),
            (
                "https://example.com/foo/./bar/baz/../qux",
                "https://example.com/foo/bar/qux/",
            ),
            ("https://example.com", "https://example.com/"),
            ("http://example.com:80/", "http://example.com/"),
            ("https://example.com:443/", "https://example.com/"),
            ("ftp://ftp.example.com:21/foo", "ftp://ftp.example.com/foo"),
            ("https://example.com/foo", "https://example.com/foo/"),
            (
                "https://example.com/foo/bar.html",
                "https://example.com/foo/bar.html",
            ),
            (
                "https://example.com/style.css",
                "https://example.com/style.css",
            ),
            (
                "https://example.com/index.html",
                "https://example.com/index.html",
            ),
            (
                "https://example.com/foo//bar",
                "https://example.com/foo/bar/",
            ),
            ("https://example.com/foo?", "https://example.com/foo/"),
            ("https://example.com/foo#", "https://example.com/foo/"),
            ("", "."),
            ("foo", "foo/"),
            ("./foo", "foo/"),
            ("../foo", "../foo/"),
        ];

        for (input, expected) in CASES {
            let url = Url::from(input).normalize();
            let result = url.as_str();
            assert_eq!(result, expected, "{input:?}");
        }
    }
}
