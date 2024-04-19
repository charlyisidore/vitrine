//! Parse and manipulate URL paths.

use serde::{Deserialize, Serialize};

/// An URL path.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
            },
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
            ("/../..", &[
                Component::RootDir,
                Component::ParentDir,
                Component::ParentDir,
            ]),
            ("/../../", &[
                Component::RootDir,
                Component::ParentDir,
                Component::ParentDir,
            ]),
            ("foo", &[Component::Normal("foo")]),
            ("foo/bar", &[
                Component::Normal("foo"),
                Component::Normal("bar"),
            ]),
            ("foo//bar", &[
                Component::Normal("foo"),
                Component::Normal("bar"),
            ]),
            ("foo/./bar", &[
                Component::Normal("foo"),
                Component::CurDir,
                Component::Normal("bar"),
            ]),
            ("foo././bar", &[
                Component::Normal("foo."),
                Component::CurDir,
                Component::Normal("bar"),
            ]),
            ("foo/", &[Component::Normal("foo")]),
            ("foo/.", &[Component::Normal("foo"), Component::CurDir]),
            ("foo/..", &[Component::Normal("foo"), Component::ParentDir]),
            ("/foo/bar", &[
                Component::RootDir,
                Component::Normal("foo"),
                Component::Normal("bar"),
            ]),
            ("/foo/bar/./baz", &[
                Component::RootDir,
                Component::Normal("foo"),
                Component::Normal("bar"),
                Component::CurDir,
                Component::Normal("baz"),
            ]),
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
