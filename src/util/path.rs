//! Utility functions for paths.

use std::path::{Path, PathBuf};

/// Extend [`std::path::Path`] with utility methods.
pub(crate) trait PathExt {
    /// Normalize the path by removing unnecessary separators and `.` and `..`
    /// components.
    ///
    /// This method calls [`normalize_path`] under the hood.
    fn normalize(&self) -> PathBuf;
}

impl<T> PathExt for T
where
    T: AsRef<Path>,
{
    fn normalize(&self) -> PathBuf {
        self::normalize_path(self)
    }
}

/// Normalize a path by removing unnecessary separators and `.` and `..`
/// components.
///
/// Unlike [`std::fs::canonicalize`], symbolic links are not resolved. Hence,
/// this function is similar to [`path.normalize()`][js] (Node.js) and
/// [`os.path.normpath()`][py] (Python).
///
/// Differences from:
/// - [`path.normalize()`][js]: trailing slashes are not preserved (e.g. `foo/`
///   returns `foo`).
/// - [`os.path.normpath()`][py]: two leading slashes are not preserved (e.g.
///   `//foo` returns `/foo`).
///
/// [js]: https://nodejs.org/api/path.html#pathnormalizepath
/// [py]: https://docs.python.org/3/library/os.path.html#os.path.normpath
#[cfg(unix)]
pub(crate) fn normalize_path<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    use std::path::Component;

    let path = path.as_ref();

    if path.as_os_str().is_empty() {
        return PathBuf::from(Component::CurDir.as_os_str());
    }

    let has_root = path.has_root();
    let mut result: Vec<Component> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::Normal(_) => {
                result.push(component);
            },
            Component::ParentDir => {
                if !has_root && result.is_empty()
                    || result
                        .last()
                        .map(|c| match c {
                            Component::ParentDir => true,
                            _ => false,
                        })
                        .unwrap_or(false)
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
        PathBuf::from(Component::RootDir.as_os_str())
    } else {
        PathBuf::new()
    };

    let result = result.into_iter().fold(root, |mut p, c| {
        p.push(c);
        p
    });

    if result.as_os_str().is_empty() {
        return PathBuf::from(Component::CurDir.as_os_str());
    }

    result
}

#[cfg(test)]
mod tests {
    #[test]
    #[cfg(unix)]
    fn normalize_path_unix() {
        const CASES: [(&str, &str); 67] = [
            ("", "."),
            ("/", "/"),
            ("/.", "/"),
            ("/./", "/"),
            ("/.//.", "/"),
            ("/foo", "/foo"),
            ("/foo/bar", "/foo/bar"),
            // Node.js: "/"
            // Python: "//"
            ("//", "/"),
            ("///", "/"),
            // Node.js: "/foo/bar/"
            // Python: "/foo/bar"
            ("///foo/.//bar//", "/foo/bar"),
            // Node.js: "/foo/baz/"
            // Python: "/foo/baz"
            ("///foo/.//bar//.//..//.//baz///", "/foo/baz"),
            ("///..//./foo/.//bar", "/foo/bar"),
            (".", "."),
            (".//.", "."),
            ("..", ".."),
            // Node.js: "../"
            // Python: ".."
            ("../", ".."),
            ("../foo", "../foo"),
            ("../../foo", "../../foo"),
            ("../foo/../bar", "../bar"),
            ("../../foo/../bar/./baz/boom/..", "../../bar/baz"),
            ("/..", "/"),
            ("/..", "/"),
            ("/../", "/"),
            ("/..//", "/"),
            // Node.js: "/"
            // Python: "//"
            ("//.", "/"),
            // Node.js: "/"
            // Python: "//"
            ("//..", "/"),
            // Node.js: "/..."
            // Python: "//..."
            ("//...", "/..."),
            // Node.js: "/foo"
            // Python: "//foo"
            ("//../foo", "/foo"),
            // Node.js: "/foo"
            // Python: "//foo"
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
            // Node.js: "/"
            // Python: "//"
            ("//foo/..", "/"),
            // Node.js: "/"
            // Python: "//"
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
            // Node.js: "bar/"
            // Python: "bar"
            ("bar/foo../../", "bar"),
            ("bar/foo../..", "bar"),
            ("bar/foo../../baz", "bar/baz"),
            // Node.js: "bar/foo../"
            // Python: "bar/foo.."
            ("bar/foo../", "bar/foo.."),
            ("bar/foo..", "bar/foo.."),
            ("../foo../../../bar", "../../bar"),
            ("../.../.././.../../../bar", "../../bar"),
            ("../../../foo/../../../bar", "../../../../../bar"),
            // Node.js: "../../../../../../"
            // Python: "../../../../../.."
            ("../../../foo/../../../bar/../../", "../../../../../.."),
            // Node.js: "../../"
            // Python: "../.."
            ("../foobar/barfoo/foo/../../../bar/../../", "../.."),
            ("../.../../foobar/../../../bar/../../baz", "../../../../baz"),
            ("foo/bar\\baz", "foo/bar\\baz"),
        ];

        for (input, expected) in CASES {
            let result = super::normalize_path(input);
            assert_eq!(
                result.to_str().unwrap(),
                expected,
                "\nnormalize_path({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
