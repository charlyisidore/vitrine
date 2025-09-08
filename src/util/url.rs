//! Parse and manipulate URLs.
//!
//! This module provides the [`Url`] type for working with URLs.

use self::{authority::Authority, path::Path};
pub use self::{
    authority::Authority as UrlAuthority, path::Path as UrlPath, query::Query as UrlQuery,
};

/// An owned and mutable URL.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

    /// Return the authority component as a [`UrlAuthority`], if any.
    pub fn authority(&self) -> Option<UrlAuthority> {
        self.authority_str().map(UrlAuthority::from)
    }

    /// Return the authority component as a string, if any.
    pub fn authority_str(&self) -> Option<&str> {
        self.components().find_map(|component| match component {
            Component::Authority(s) => Some(s),
            _ => None,
        })
    }

    /// Return the path component as a [`UrlPath`].
    pub fn path(&self) -> UrlPath {
        UrlPath::from(self.path_str())
    }

    /// Return the path component as a string.
    pub fn path_str(&self) -> &str {
        self.components()
            .find_map(|component| match component {
                Component::Path(s) => Some(s),
                _ => None,
            })
            .expect("must have path component")
    }

    /// Return the query component as a [`UrlQuery`], if any.
    pub fn query(&self) -> Option<UrlQuery> {
        self.query_str().map(UrlQuery::from)
    }

    /// Return the query component as a string, if any.
    pub fn query_str(&self) -> Option<&str> {
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

    /// Replace the scheme component.
    pub fn set_scheme(&mut self, scheme: Option<&str>) {
        self.0 = Self::str_from_components(
            scheme,
            self.authority_str(),
            self.path_str(),
            self.query_str(),
            self.fragment(),
        )
    }

    /// Replace the authority component.
    pub fn set_authority(&mut self, authority: Option<&str>) {
        self.0 = Self::str_from_components(
            self.scheme(),
            authority,
            self.path_str(),
            self.query_str(),
            self.fragment(),
        )
    }

    /// Replace the path component.
    pub fn set_path(&mut self, path: &str) {
        self.0 = Self::str_from_components(
            self.scheme(),
            self.authority_str(),
            path,
            self.query_str(),
            self.fragment(),
        )
    }

    /// Replace the query component.
    pub fn set_query(&mut self, query: Option<&str>) {
        self.0 = Self::str_from_components(
            self.scheme(),
            self.authority_str(),
            self.path_str(),
            query,
            self.fragment(),
        )
    }

    /// Replace the fragment component.
    pub fn set_fragment(&mut self, fragment: Option<&str>) {
        self.0 = Self::str_from_components(
            self.scheme(),
            self.authority_str(),
            self.path_str(),
            self.query_str(),
            fragment,
        )
    }

    /// Return a [`Url`] with the scheme component replaced.
    pub fn with_scheme(&self, scheme: Option<&str>) -> Self {
        let mut url = self.clone();
        url.set_scheme(scheme);
        url
    }

    /// Return a [`Url`] with the scheme component replaced.
    pub fn with_authority(&self, authority: Option<&str>) -> Self {
        let mut url = self.clone();
        url.set_authority(authority);
        url
    }

    /// Return a [`Url`] with the scheme component replaced.
    pub fn with_path(&self, path: &str) -> Self {
        let mut url = self.clone();
        url.set_path(path);
        url
    }

    /// Return a [`Url`] with the scheme component replaced.
    pub fn with_query(&self, query: Option<&str>) -> Self {
        let mut url = self.clone();
        url.set_query(query);
        url
    }

    /// Return a [`Url`] with the scheme component replaced.
    pub fn with_fragment(&self, fragment: Option<&str>) -> Self {
        let mut url = self.clone();
        url.set_fragment(fragment);
        url
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
                }
                Component::Authority(s) => {
                    url.push_str("//");
                    let s = Authority::from(s).normalize(scheme);
                    url.push_str(s.as_str());
                    absolute = true;
                }
                Component::Path(s) => {
                    let s = Path::from(s).normalize_in_url(absolute);
                    url.push_str(s.as_str());
                }
                Component::Query(s) => {
                    if !s.is_empty() {
                        url.push('?');
                        url.push_str(s);
                    }
                }
                Component::Fragment(s) => {
                    if !s.is_empty() {
                        url.push('#');
                        url.push_str(s);
                    }
                }
            }
        }
        Self(url)
    }

    /// Create a URL string from given components.
    fn str_from_components(
        scheme: Option<&str>,
        authority: Option<&str>,
        path: &str,
        query: Option<&str>,
        fragment: Option<&str>,
    ) -> String {
        let mut url = String::new();
        if let Some(s) = scheme {
            url.push_str(s);
            url.push(':');
        }
        if let Some(s) = authority {
            url.push_str("//");
            url.push_str(s);
        }
        url.push_str(path);
        if let Some(s) = query {
            url.push('?');
            url.push_str(s);
        }
        if let Some(s) = fragment {
            url.push('#');
            url.push_str(s);
        }
        url
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
            }
            '/' if i == 0 && self.url.starts_with("//") => {
                self.url = &self.url[2..];
                self.parse_authority()
            }
            '\0' => {
                self.state = State::End;
                Some(Component::Path(self.url))
            }
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
            }
            '?' => {
                self.url = &self.url[i + 1..];
                self.state = State::Query;
            }
            '#' => {
                self.url = &self.url[i + 1..];
                self.state = State::Fragment;
            }
            '\0' => {
                self.url = &self.url[self.url.len()..];
                self.state = State::Path;
            }
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
            }
            '#' => {
                self.url = &self.url[i + 1..];
                self.state = State::Fragment;
            }
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
            }
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

mod authority {
    /// An URL authority component.
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Authority(String);

    impl Authority {
        /// Return a reference to the inner [`str`] slice.
        pub fn as_str(&self) -> &str {
            &self.0
        }

        /// Consume the [`Authority`] and return the inner [`String`].
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

        /// Return the userinfo component, if any.
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
        ///
        /// This method transforms the host into lowercase, and removes the port
        /// component when it matches the default port of the specified scheme. The
        /// userinfo component is untouched.
        pub fn normalize(&self, scheme: Option<&str>) -> Self {
            let mut authority = String::new();
            for component in self.components() {
                match component {
                    Component::Userinfo(s) => {
                        authority.push_str(s);
                        authority.push('@');
                    }
                    Component::Host(s) => authority.push_str(&s.to_ascii_lowercase()),
                    Component::Port(s) => {
                        if !s.is_empty()
                            && scheme
                                .filter(|scheme| {
                                    matches!(
                                        (scheme.to_ascii_lowercase().as_str(), s),
                                        ("ftp", "21")
                                            | ("http", "80")
                                            | ("https", "443")
                                            | ("ws", "80")
                                            | ("wss", "443")
                                    )
                                })
                                .is_none()
                        {
                            authority.push(':');
                            authority.push_str(s);
                        }
                    }
                }
            }
            Self(authority)
        }
    }

    impl std::fmt::Display for Authority {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl AsRef<str> for Authority {
        fn as_ref(&self) -> &str {
            &self.0
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

    /// An iterator over the [`Component`]s of an [`Authority`].
    #[derive(Debug)]
    pub struct Components<'a> {
        authority: &'a str,
        state: State,
    }

    /// An URL authority component.
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
        /// Parsing the userinfo or host component.
        Start,
        /// Parsing the host component.
        Host,
        /// Parsing the port component.
        Port,
        /// Finished parsing.
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
                }
                ']' => {
                    inside_brackets = false;
                    false
                }
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

    #[cfg(test)]
    mod tests {
        use super::{Authority, Component};

        #[test]
        fn components() {
            const CASES: [(&str, &[Component]); 8] = [
                ("example.com", &[Component::Host("example.com")]),
                ("ftp.is.co.za", &[Component::Host("ftp.is.co.za")]),
                ("[2001:db8::7]", &[Component::Host("[2001:db8::7]")]),
                (
                    "192.0.2.16:80",
                    &[Component::Host("192.0.2.16"), Component::Port("80")],
                ),
                (
                    "example.com:8042",
                    &[Component::Host("example.com"), Component::Port("8042")],
                ),
                (
                    "User@example.com",
                    &[Component::Userinfo("User"), Component::Host("example.com")],
                ),
                (
                    "User@[2001:db8::7]:8042",
                    &[
                        Component::Userinfo("User"),
                        Component::Host("[2001:db8::7]"),
                        Component::Port("8042"),
                    ],
                ),
                (
                    "example.com:",
                    &[Component::Host("example.com"), Component::Port("")],
                ),
            ];

            for (input, expected) in CASES {
                let authority = Authority::from(input);
                let result: Vec<_> = authority.components().collect();
                assert_eq!(result, expected, "{input:?}");
            }
        }

        #[test]
        fn normalize() {
            const CASES: [(Option<&str>, &str, &str); 6] = [
                (None, "EXAMPLE.com", "example.com"),
                (None, "example.com", "example.com"),
                (None, "example.com:", "example.com"),
                (Some("http"), "example.com:80", "example.com"),
                (Some("http"), "example.com:8000", "example.com:8000"),
                (Some("https"), "example.com:443", "example.com"),
            ];

            for (scheme, input, expected) in CASES {
                let authority = Authority::from(input);
                let result = authority.normalize(scheme);
                assert_eq!(result.as_str(), expected, "{input:?}");
            }
        }
    }
}

mod path {
    use serde::Serialize;

    /// An URL path.
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
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
            Segments(&self.0)
        }

        /// Return an iterator over the segments of the path, preserving any
        /// trailing slash.
        pub fn segments_with_slash(&self) -> SegmentsWithSlash {
            SegmentsWithSlash(&self.0)
        }

        /// Return an iterator over the [`Component`]s of the path.
        pub fn components(&self) -> Components {
            Components(self.segments_with_slash())
        }

        /// Return the path without the final component, if any.
        pub fn parent(&self) -> Self {
            let mut path = self.clone();
            path.pop();
            path
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

        /// Remove the last segment from `self`.
        pub fn pop(&mut self) -> bool {
            if self.0.is_empty() {
                false
            } else if let Some(len) = self.0.trim_end_matches('/').rfind('/') {
                self.0.truncate(len + 1);
                true
            } else if !self.0.starts_with('/') {
                self.0.clear();
                true
            } else {
                false
            }
        }

        /// Append a segment to `self`.
        pub fn push(&mut self, segment: impl AsRef<str>) {
            let segment = segment.as_ref();
            if !self.0.is_empty() && !self.0.ends_with('/') && !segment.starts_with('/') {
                self.0.push('/');
            }
            self.0.push_str(segment);
        }

        /// Append a segment to `self` and returns the result.
        pub fn join(&self, segment: impl AsRef<str>) -> Self {
            let mut path = self.clone();
            path.push(segment);
            path
        }

        /// Remove a prefix.
        pub fn strip_prefix(&self, prefix: impl AsRef<str>) -> Option<Self> {
            self.0.strip_prefix(prefix.as_ref()).map(Self::from)
        }

        /// Remove a suffix.
        pub fn strip_suffix(&self, suffix: impl AsRef<str>) -> Option<Self> {
            self.0.strip_suffix(suffix.as_ref()).map(Self::from)
        }

        /// Normalize the path.
        pub fn normalize(&self) -> Self {
            if self.0.is_empty() {
                return Self::new();
            }
            self.normalize_in_url(false)
        }

        /// Convert the URL path to a [`std::path::PathBuf`].
        pub fn to_std_path_buf(&self) -> std::path::PathBuf {
            let mut path = std::path::PathBuf::new();
            for component in self.components() {
                match component {
                    Component::RootDir => path.push(std::path::Component::RootDir),
                    Component::CurDir => path.push(std::path::Component::CurDir),
                    Component::ParentDir => path.push(std::path::Component::ParentDir),
                    Component::Normal(s) => path.push(s),
                }
            }
            path
        }

        /// Normalize the path within a URL.
        pub(super) fn normalize_in_url(&self, absolute: bool) -> Self {
            if self.0.is_empty() {
                return Self::from(
                    if absolute {
                        Component::RootDir
                    } else {
                        Component::CurDir
                    }
                    .as_str(),
                );
            }

            let has_root = absolute || self.0.starts_with('/');
            let has_trailing_slash = self.0.ends_with('/');
            let mut result: Vec<Component> = Vec::new();

            for component in self.components() {
                match component {
                    Component::Normal(_) => result.push(component),
                    Component::ParentDir => {
                        if !has_root && result.is_empty()
                            || result
                                .last()
                                .is_some_and(|c| matches!(c, Component::ParentDir))
                        {
                            result.push(component);
                        } else {
                            result.pop();
                        }
                    }
                    Component::RootDir | Component::CurDir => {}
                }
            }

            let root = if has_root {
                String::from(Component::RootDir.as_str())
            } else {
                String::with_capacity(self.0.len())
            };

            let result = result.iter().enumerate().fold(root, |mut p, (i, c)| {
                p.push_str(c.as_str());
                if i + 1 < result.len() || has_trailing_slash {
                    p.push('/');
                }
                p
            });

            if result.is_empty() {
                return Self::from(Component::CurDir.as_str());
            }

            Self(result)
        }
    }

    impl std::fmt::Display for Path {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl AsRef<str> for Path {
        fn as_ref(&self) -> &str {
            &self.0
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
    pub struct Segments<'a>(&'a str);

    impl<'a> Iterator for Segments<'a> {
        type Item = &'a str;

        fn next(&mut self) -> Option<Self::Item> {
            if self.0.is_empty() {
                return None;
            }
            self.0 = self.0.trim_start_matches('/');
            if let Some(i) = self.0.find('/') {
                let segment = &self.0[..i];
                self.0 = &self.0[i..];
                Some(segment)
            } else {
                let segment = self.0;
                self.0 = &self.0[self.0.len()..];
                Some(segment)
            }
        }
    }

    /// An iterator over the segments of a [`Path`], preserving trailing slashes.
    #[derive(Debug)]
    pub struct SegmentsWithSlash<'a>(&'a str);

    impl<'a> Iterator for SegmentsWithSlash<'a> {
        type Item = &'a str;

        fn next(&mut self) -> Option<Self::Item> {
            if self.0.is_empty() {
                return None;
            }
            if let Some(i) = self.0.find('/') {
                let segment = &self.0[..i + 1];
                self.0 = self.0[i + 1..].trim_start_matches('/');
                Some(segment)
            } else {
                let segment = self.0;
                self.0 = &self.0[self.0.len()..];
                Some(segment)
            }
        }
    }

    /// An iterator over the [`Component`]s of a [`Path`].
    #[derive(Debug)]
    pub struct Components<'a>(SegmentsWithSlash<'a>);

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
            self.0.next().map(|segment| match segment {
                "/" => Component::RootDir,
                "." | "./" => Component::CurDir,
                ".." | "../" => Component::ParentDir,
                s => {
                    let s = s.strip_suffix('/').unwrap_or(s);
                    debug_assert!(!s.ends_with('/'));
                    Component::Normal(s)
                }
            })
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

    #[cfg(test)]
    mod tests {
        use super::{Component, Path};

        #[test]
        fn segments() {
            const CASES: [(&str, &[&str]); 24] = [
                ("", &[]),
                (".", &["."]),
                ("..", &[".."]),
                ("./", &[".", ""]),
                ("../", &["..", ""]),
                ("../..", &["..", ".."]),
                ("../../", &["..", "..", ""]),
                ("/", &[""]),
                ("/.", &["."]),
                ("/..", &[".."]),
                ("/./", &[".", ""]),
                ("/../", &["..", ""]),
                ("/../..", &["..", ".."]),
                ("/../../", &["..", "..", ""]),
                ("foo", &["foo"]),
                ("foo/bar", &["foo", "bar"]),
                ("foo//bar", &["foo", "bar"]),
                ("foo/./bar", &["foo", ".", "bar"]),
                ("foo././bar", &["foo.", ".", "bar"]),
                ("foo/", &["foo", ""]),
                ("foo/.", &["foo", "."]),
                ("foo/..", &["foo", ".."]),
                ("/foo/bar", &["foo", "bar"]),
                ("/foo/bar/./baz", &["foo", "bar", ".", "baz"]),
            ];

            for (input, expected) in CASES {
                let path = Path::from(input);
                let result: Vec<_> = path.segments().collect();
                assert_eq!(result, expected, "{input:?}");
            }
        }

        #[test]
        fn segments_with_slash() {
            const CASES: [(&str, &[&str]); 24] = [
                ("", &[]),
                (".", &["."]),
                ("..", &[".."]),
                ("./", &["./"]),
                ("../", &["../"]),
                ("../..", &["../", ".."]),
                ("../../", &["../", "../"]),
                ("/", &["/"]),
                ("/.", &["/", "."]),
                ("/..", &["/", ".."]),
                ("/./", &["/", "./"]),
                ("/../", &["/", "../"]),
                ("/../..", &["/", "../", ".."]),
                ("/../../", &["/", "../", "../"]),
                ("foo", &["foo"]),
                ("foo/bar", &["foo/", "bar"]),
                ("foo//bar", &["foo/", "bar"]),
                ("foo/./bar", &["foo/", "./", "bar"]),
                ("foo././bar", &["foo./", "./", "bar"]),
                ("foo/", &["foo/"]),
                ("foo/.", &["foo/", "."]),
                ("foo/..", &["foo/", ".."]),
                ("/foo/bar", &["/", "foo/", "bar"]),
                ("/foo/bar/./baz", &["/", "foo/", "bar/", "./", "baz"]),
            ];

            for (input, expected) in CASES {
                let path = Path::from(input);
                let result: Vec<_> = path.segments_with_slash().collect();
                assert_eq!(result, expected, "{input:?}");
            }
        }

        #[test]
        fn components() {
            const CASES: [(&str, &[Component]); 24] = [
                ("", &[]),
                (".", &[Component::CurDir]),
                ("..", &[Component::ParentDir]),
                ("./", &[Component::CurDir]),
                ("../", &[Component::ParentDir]),
                ("../..", &[Component::ParentDir, Component::ParentDir]),
                ("../../", &[Component::ParentDir, Component::ParentDir]),
                ("/", &[Component::RootDir]),
                ("/.", &[Component::RootDir, Component::CurDir]),
                ("/..", &[Component::RootDir, Component::ParentDir]),
                ("/./", &[Component::RootDir, Component::CurDir]),
                ("/../", &[Component::RootDir, Component::ParentDir]),
                (
                    "/../..",
                    &[
                        Component::RootDir,
                        Component::ParentDir,
                        Component::ParentDir,
                    ],
                ),
                (
                    "/../../",
                    &[
                        Component::RootDir,
                        Component::ParentDir,
                        Component::ParentDir,
                    ],
                ),
                ("foo", &[Component::Normal("foo")]),
                (
                    "foo/bar",
                    &[Component::Normal("foo"), Component::Normal("bar")],
                ),
                (
                    "foo//bar",
                    &[Component::Normal("foo"), Component::Normal("bar")],
                ),
                (
                    "foo/./bar",
                    &[
                        Component::Normal("foo"),
                        Component::CurDir,
                        Component::Normal("bar"),
                    ],
                ),
                (
                    "foo././bar",
                    &[
                        Component::Normal("foo."),
                        Component::CurDir,
                        Component::Normal("bar"),
                    ],
                ),
                ("foo/", &[Component::Normal("foo")]),
                ("foo/.", &[Component::Normal("foo"), Component::CurDir]),
                ("foo/..", &[Component::Normal("foo"), Component::ParentDir]),
                (
                    "/foo/bar",
                    &[
                        Component::RootDir,
                        Component::Normal("foo"),
                        Component::Normal("bar"),
                    ],
                ),
                (
                    "/foo/bar/./baz",
                    &[
                        Component::RootDir,
                        Component::Normal("foo"),
                        Component::Normal("bar"),
                        Component::CurDir,
                        Component::Normal("baz"),
                    ],
                ),
            ];

            for (input, expected) in CASES {
                let path = Path::from(input);
                let result: Vec<_> = path.components().collect();
                assert_eq!(result, expected, "{input:?}");
            }
        }

        #[test]
        fn pop() {
            const CASES: [(&str, &str); 12] = [
                ("", ""),
                ("foo", ""),
                ("foo/", ""),
                ("foo/bar", "foo/"),
                ("foo/bar/", "foo/"),
                ("foo/../bar", "foo/../"),
                ("/", "/"),
                ("/foo", "/"),
                ("/foo/", "/"),
                ("/foo/bar", "/foo/"),
                ("/foo/bar/", "/foo/"),
                ("/foo/../bar", "/foo/../"),
            ];

            for (input, expected) in CASES {
                let mut result = Path::from(input);
                result.pop();
                assert_eq!(result.as_str(), expected, "{input:?}");
            }
        }

        #[test]
        fn push() {
            const CASES: [(&str, &str, &str); 8] = [
                ("", "foo", "foo"),
                ("", "foo/", "foo/"),
                ("..", "foo", "../foo"),
                ("foo", "bar", "foo/bar"),
                ("foo/", "bar", "foo/bar"),
                ("/", "foo", "/foo"),
                ("/", "foo/", "/foo/"),
                ("/foo", "bar", "/foo/bar"),
            ];

            for (input, segment, expected) in CASES {
                let mut result = Path::from(input);
                result.push(segment);
                assert_eq!(result.as_str(), expected, "{input:?}");
            }
        }

        #[test]
        fn normalize() {
            const CASES: [(&str, &str); 66] = [
                ("", ""),
                ("/", "/"),
                ("/.", "/"),
                ("/./", "/"),
                ("/.//.", "/"),
                ("/foo", "/foo"),
                ("/foo/bar", "/foo/bar"),
                ("//", "/"),
                ("///", "/"),
                ("///foo/.//bar//", "/foo/bar/"),
                ("///foo/.//bar//.//..//.//baz///", "/foo/baz/"),
                ("///..//./foo/.//bar", "/foo/bar"),
                (".", "."),
                (".//.", "."),
                ("..", ".."),
                ("../", "../"),
                ("../foo", "../foo"),
                ("../../foo", "../../foo"),
                ("../foo/../bar", "../bar"),
                ("../../foo/../bar/./baz/boom/..", "../../bar/baz"),
                ("/..", "/"),
                ("/../", "/"),
                ("/..//", "/"),
                ("//.", "/"),
                ("//..", "/"),
                ("//...", "/..."),
                ("//../foo", "/foo"),
                ("//../../foo", "/foo"),
                ("/../foo", "/foo"),
                ("/../../foo", "/foo"),
                ("/../foo/../", "/"),
                ("/../foo/../bar", "/bar"),
                ("/../../foo/../bar/./baz/boom/..", "/bar/baz"),
                ("/../../foo/../bar/./baz/boom/.", "/bar/baz/boom"),
                ("foo/../bar/baz", "bar/baz"),
                ("foo/../../bar/baz", "../bar/baz"),
                ("foo/../../../bar/baz", "../../bar/baz"),
                ("foo///../bar/.././../baz/boom", "../baz/boom"),
                ("foo/bar/../..///../../baz/boom", "../../baz/boom"),
                ("/foo/..", "/"),
                ("/foo/../..", "/"),
                ("//foo/..", "/"),
                ("//foo/../..", "/"),
                ("///foo/..", "/"),
                ("///foo/../..", "/"),
                ("////foo/..", "/"),
                ("/////foo/..", "/"),
                ("./fixtures///b/../b/c.rs", "fixtures/b/c.rs"),
                ("/foo/../../../bar", "/bar"),
                ("a//b//../b", "a/b"),
                ("a//b//./c", "a/b/c"),
                ("a//b//.", "a/b"),
                ("/a/b/c/../../../x/y/z", "/x/y/z"),
                ("///..//./foo/.//bar", "/foo/bar"),
                ("bar/foo../../", "bar/"),
                ("bar/foo../..", "bar"),
                ("bar/foo../../baz", "bar/baz"),
                ("bar/foo../", "bar/foo../"),
                ("bar/foo..", "bar/foo.."),
                ("../foo../../../bar", "../../bar"),
                ("../.../.././.../../../bar", "../../bar"),
                ("../../../foo/../../../bar", "../../../../../bar"),
                ("../../../foo/../../../bar/../../", "../../../../../../"),
                ("../foobar/barfoo/foo/../../../bar/../../", "../../"),
                ("../.../../foobar/../../../bar/../../baz", "../../../../baz"),
                ("foo/bar\\baz", "foo/bar\\baz"),
            ];

            for (input, expected) in CASES {
                let path = Path::from(input).normalize();
                let result = path.as_str();
                assert_eq!(result, expected, "{input:?}");
            }
        }

        #[test]
        #[cfg(unix)]
        fn to_std_path_buf() {
            const CASES: [(&str, &str); 10] = [
                ("", ""),
                (".", "."),
                ("..", ".."),
                ("foo", "foo"),
                ("foo/bar", "foo/bar"),
                ("foo/./bar", "foo/./bar"),
                ("foo/../bar", "foo/../bar"),
                ("/", "/"),
                ("/foo", "/foo"),
                ("/foo/bar", "/foo/bar"),
            ];

            for (input, expected) in CASES {
                let result = Path::from(input).to_std_path_buf();
                let expected = std::path::PathBuf::from(expected);
                assert_eq!(result, expected, "{input:?}");
            }
        }
    }
}

mod query {
    /// An URL query component.
    #[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct Query(String);

    impl Query {
        /// Return a reference to the inner [`str`] slice.
        pub fn as_str(&self) -> &str {
            &self.0
        }

        /// Consume the [`Query`] and return the inner [`String`].
        pub fn into_string(self) -> String {
            self.0
        }

        /// Return an iterator over the parameters of the query.
        pub fn parameters(&self) -> Parameters {
            Parameters(&self.0)
        }

        /// Check if a parameter key exists.
        pub fn has(&self, key: impl AsRef<str>) -> bool {
            let key = key.as_ref();
            self.parameters().any(|(k, _)| k == key)
        }

        /// Return the value of a parameter given its key, if any.
        pub fn get(&self, key: impl AsRef<str>) -> Option<&str> {
            let key = key.as_ref();
            self.parameters()
                .find(|(k, _)| *k == key)
                .and_then(|(_, v)| v)
        }
    }

    impl std::fmt::Display for Query {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl AsRef<str> for Query {
        fn as_ref(&self) -> &str {
            &self.0
        }
    }

    impl<T> From<T> for Query
    where
        T: Into<String>,
    {
        fn from(value: T) -> Self {
            Self(value.into())
        }
    }

    /// An iterator over the parameters of a [`Query`].
    #[derive(Debug)]
    pub struct Parameters<'a>(&'a str);

    impl<'a> Iterator for Parameters<'a> {
        type Item = (&'a str, Option<&'a str>);

        fn next(&mut self) -> Option<Self::Item> {
            if self.0.is_empty() {
                None
            } else {
                let parameter = if let Some(i) = self.0.find('&') {
                    let parameter = &self.0[..i];
                    self.0 = &self.0[i + 1..];
                    parameter
                } else {
                    let parameter = self.0;
                    self.0 = &self.0[self.0.len()..];
                    parameter
                };
                parameter
                    .split_once('=')
                    .map(|(k, v)| (k, Some(v)))
                    .or(Some((parameter, None)))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::Query;

        #[test]
        fn parameters() {
            const CASES: [(&str, &[(&str, Option<&str>)]); 5] = [
                ("", &[]),
                ("fred", &[("fred", None)]),
                ("name=ferret", &[("name", Some("ferret"))]),
                (
                    "name=ferret&color=purple",
                    &[("name", Some("ferret")), ("color", Some("purple"))],
                ),
                ("objectClass?one", &[("objectClass?one", None)]),
            ];

            for (input, expected) in CASES {
                let query = Query::from(input);
                let result: Vec<_> = query.parameters().collect();
                assert_eq!(result, expected, "{input:?}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Component, Url};

    #[test]
    fn components() {
        const CASES: [(&str, &[Component]); 37] = [
            (
                "https://example.com",
                &[
                    Component::Scheme("https"),
                    Component::Authority("example.com"),
                    Component::Path(""),
                ],
            ),
            (
                "ftp://ftp.is.co.za/rfc/rfc1808.txt",
                &[
                    Component::Scheme("ftp"),
                    Component::Authority("ftp.is.co.za"),
                    Component::Path("/rfc/rfc1808.txt"),
                ],
            ),
            (
                "http://www.ietf.org/rfc/rfc2396.txt",
                &[
                    Component::Scheme("http"),
                    Component::Authority("www.ietf.org"),
                    Component::Path("/rfc/rfc2396.txt"),
                ],
            ),
            (
                "ldap://[2001:db8::7]/c=GB?objectClass?one",
                &[
                    Component::Scheme("ldap"),
                    Component::Authority("[2001:db8::7]"),
                    Component::Path("/c=GB"),
                    Component::Query("objectClass?one"),
                ],
            ),
            (
                "mailto:John.Doe@example.com",
                &[
                    Component::Scheme("mailto"),
                    Component::Path("John.Doe@example.com"),
                ],
            ),
            (
                "news:comp.infosystems.www.servers.unix",
                &[
                    Component::Scheme("news"),
                    Component::Path("comp.infosystems.www.servers.unix"),
                ],
            ),
            (
                "tel:+1-816-555-1212",
                &[Component::Scheme("tel"), Component::Path("+1-816-555-1212")],
            ),
            (
                "telnet://192.0.2.16:80/",
                &[
                    Component::Scheme("telnet"),
                    Component::Authority("192.0.2.16:80"),
                    Component::Path("/"),
                ],
            ),
            (
                "urn:oasis:names:specification:docbook:dtd:xml:4.1.2",
                &[
                    Component::Scheme("urn"),
                    Component::Path("oasis:names:specification:docbook:dtd:xml:4.1.2"),
                ],
            ),
            (
                "foo://example.com:8042/over/there?name=ferret#nose",
                &[
                    Component::Scheme("foo"),
                    Component::Authority("example.com:8042"),
                    Component::Path("/over/there"),
                    Component::Query("name=ferret"),
                    Component::Fragment("nose"),
                ],
            ),
            (
                "urn:example:animal:ferret:nose",
                &[
                    Component::Scheme("urn"),
                    Component::Path("example:animal:ferret:nose"),
                ],
            ),
            (
                "http://a/b/c/d;p?q",
                &[
                    Component::Scheme("http"),
                    Component::Authority("a"),
                    Component::Path("/b/c/d;p"),
                    Component::Query("q"),
                ],
            ),
            (
                "data:,A%20brief%20note",
                &[
                    Component::Scheme("data"),
                    Component::Path(",A%20brief%20note"),
                ],
            ),
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
            (
                "g?y#s",
                &[
                    Component::Path("g"),
                    Component::Query("y"),
                    Component::Fragment("s"),
                ],
            ),
            (";x", &[Component::Path(";x")]),
            ("g;x", &[Component::Path("g;x")]),
            (
                "g;x?y#s",
                &[
                    Component::Path("g;x"),
                    Component::Query("y"),
                    Component::Fragment("s"),
                ],
            ),
            ("", &[Component::Path("")]),
            (".", &[Component::Path(".")]),
            ("./", &[Component::Path("./")]),
            ("..", &[Component::Path("..")]),
            ("../", &[Component::Path("../")]),
            ("../g", &[Component::Path("../g")]),
            ("../..", &[Component::Path("../..")]),
            ("../../", &[Component::Path("../../")]),
            ("../../g", &[Component::Path("../../g")]),
            (
                "//example.com",
                &[Component::Authority("example.com"), Component::Path("")],
            ),
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
