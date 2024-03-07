//! Parse and manipulate URL paths.

use serde::{Deserialize, Serialize};

/// An URL path.
#[derive(Clone, Debug, Default, Hash, PartialEq, Serialize, Deserialize)]
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

    /// Return an iterator over the [`Component`]s of the path.
    pub fn components(&self) -> Components {
        Components(self.segments())
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

    /// Remove dot segments.
    pub fn remove_dot_segments(&self) -> Self {
        let mut input = self.0.clone();
        let mut output = String::new();
        while !input.is_empty() {
            if let Some(rest) = input
                .strip_prefix("../")
                .or_else(|| input.strip_prefix("./"))
            {
                input = rest.into();
            } else if let Some(rest) = input
                .strip_prefix("/./")
                .or_else(|| input.strip_prefix("/.").filter(|s| s.is_empty()))
            {
                input = format!("/{}", rest);
            } else if let Some(rest) = input
                .strip_prefix("/../")
                .or_else(|| input.strip_prefix("/..").filter(|s| s.is_empty()))
            {
                input = format!("/{}", rest);
                if let Some((rest, _)) = output.rsplit_once('/') {
                    output = rest.into();
                } else {
                    output.clear();
                }
            } else if [".", ".."].contains(&input.as_str()) {
                input.clear();
            } else {
                let i = input
                    .char_indices()
                    .find(|(i, c)| *i != 0 && *c == '/')
                    .map(|(i, _)| i)
                    .unwrap_or(input.len());
                let (prefix, suffix) = input.split_at(i);
                output.push_str(prefix);
                input = suffix.into();
            }
        }
        Self(output)
    }

    /// Normalize the path.
    pub fn normalize(&self) -> Self {
        if self.0.is_empty() {
            return Self::new();
        }

        let has_root = self.0.starts_with('/');
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

    /// Normalize the path.
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
pub struct Components<'a>(Segments<'a>);

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
            s => Component::Normal(s.trim_end_matches('/')),
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
            let result: Vec<_> = path.segments().collect();
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
    fn remove_dot_segments() {
        const CASES: [(&str, &str); 35] = [
            ("", ""),
            (".", ""),
            ("./", ""),
            ("..", ""),
            ("../", ""),
            ("/", "/"),
            ("/.", "/"),
            ("/..", "/"),
            ("./..", ""),
            ("./../", ""),
            ("../..", ""),
            ("../../", ""),
            ("foo", "foo"),
            ("./foo", "foo"),
            ("./foo/", "foo/"),
            ("././foo", "foo"),
            ("././foo/", "foo/"),
            ("../foo", "foo"),
            ("../foo/", "foo/"),
            ("../../foo", "foo"),
            ("../../foo/", "foo/"),
            ("foo/.", "foo/"),
            ("foo/./", "foo/"),
            ("foo/./.", "foo/"),
            ("foo/././", "foo/"),
            ("foo/././bar", "foo/bar"),
            ("foo/././bar/", "foo/bar/"),
            ("foo/..", "/"),
            ("foo/../", "/"),
            ("foo/../..", "/"),
            ("foo/../../", "/"),
            ("foo/../../bar", "/bar"),
            ("foo/../../bar/", "/bar/"),
            ("/a/b/c/./../../g", "/a/g"),
            ("mid/content=5/../6", "mid/6"),
        ];

        for (input, expected) in CASES {
            let path = Path::from(input).remove_dot_segments();
            let result = path.as_str();
            assert_eq!(result, expected, "{input:?}");
        }
    }
}
