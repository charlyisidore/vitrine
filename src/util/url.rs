//! Parse and manipulate URLs.
//!
//! This module provides the [`Url`] type for working with URLs.

pub mod authority;
pub mod path;

use serde::{Deserialize, Serialize};

pub use self::{authority::Authority as UrlAuthority, path::Path as UrlPath};
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

    /// Normalize the URL.
    pub fn normalize(&self) -> Self {
        let mut url = String::with_capacity(self.0.len());
        let mut scheme: Option<&str> = None;
        let mut absolute = false;
        for component in self.components() {
            match component {
                Component::Scheme(s) => {
                    url.push_str(&s.to_ascii_lowercase());
                    url.push(':');
                    scheme = Some(s);
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
                    let s = Path::from(s).normalize_in_url(absolute);
                    url.push_str(s.as_str());
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

#[cfg(test)]
mod tests {
    use super::{Component, Url};

    #[test]
    fn components() {
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
    fn normalize() {
        const CASES: [(&str, &str); 31] = [
            ("", "."),
            (".", "."),
            ("..", ".."),
            ("../", "../"),
            ("foo", "foo"),
            ("foo/", "foo/"),
            ("./foo", "foo"),
            ("./foo/", "foo/"),
            ("../foo", "../foo"),
            ("../foo/", "../foo/"),
            ("/foo//bar", "/foo/bar"),
            ("/foo//bar/", "/foo/bar/"),
            ("foo?", "foo"),
            ("foo#", "foo"),
            ("foo?bar", "foo?bar"),
            ("foo#bar", "foo#bar"),
            ("foo?bar#baz", "foo?bar#baz"),
            (
                "HTTPS://User@Example.COM/Foo",
                "https://User@example.com/Foo",
            ),
            (
                "FTP://User@ftp.Example.COM:21/Foo",
                "ftp://User@ftp.example.com/Foo",
            ),
            (
                "https://example.com/foo/./bar/baz/../qux",
                "https://example.com/foo/bar/qux",
            ),
            ("https://example.com", "https://example.com/"),
            ("http://example.com:80/", "http://example.com/"),
            ("https://example.com:443/", "https://example.com/"),
            ("ftp://ftp.example.com:21/foo", "ftp://ftp.example.com/foo"),
            ("https://example.com/foo", "https://example.com/foo"),
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
                "https://example.com/foo/bar",
            ),
            ("https://example.com/foo?", "https://example.com/foo"),
            ("https://example.com/foo#", "https://example.com/foo"),
        ];

        for (input, expected) in CASES {
            let url = Url::from(input).normalize();
            let result = url.as_str();
            assert_eq!(result, expected, "{input:?}");
        }
    }
}
