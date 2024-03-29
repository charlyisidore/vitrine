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
            Component::Normal(_) => result.push(component),
            Component::ParentDir => {
                if !has_root && result.is_empty()
                    || result
                        .last()
                        .map(|c| matches!(c, Component::ParentDir))
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

/// Normalize a path by removing unnecessary separators and `.` and `..`
/// components.
///
/// Unlike [`std::fs::canonicalize`], symbolic links are not resolved. Hence,
/// this function is similar to [`path.normalize()`][js] (Node.js) and
/// [`os.path.normpath()`][py] (Python).
///
/// Differences from:
/// - [`path.normalize()`][js]: trailing back slashes are not preserved (e.g.
///   `foo\` returns `foo`).
/// - [`os.path.normpath()`][py]: two leading back slashes are not preserved
///   (e.g. `\\foo` returns `\foo`).
///
/// [js]: https://nodejs.org/api/path.html#pathnormalizepath
/// [py]: https://docs.python.org/3/library/os.path.html#os.path.normpath
#[cfg(windows)]
pub(crate) fn normalize_path<P>(path: P) -> PathBuf
where
    P: AsRef<Path>,
{
    use std::path::{Component, PrefixComponent};

    let path = path.as_ref();

    let has_root = path.has_root();
    let mut prefix: Option<PrefixComponent> = None;
    let mut result: Vec<Component> = Vec::new();

    for component in path.components() {
        match component {
            Component::Prefix(prefix_component) => prefix = Some(prefix_component),
            Component::Normal(_) => result.push(component),
            Component::ParentDir => {
                if !has_root && result.is_empty()
                    || result
                        .last()
                        .map(|c| matches!(c, Component::ParentDir))
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

    if result.is_empty() && prefix.is_none() && !has_root {
        result.push(Component::CurDir);
    }

    let mut root = PathBuf::new();

    if let Some(prefix) = prefix {
        if let Some(prefix) = prefix.as_os_str().to_str() {
            root.push(prefix.replace('/', "\\"));
        } else {
            root.push(prefix.as_os_str());
        }
    }

    if has_root {
        root.push(Component::RootDir);
    }

    result.into_iter().fold(root, |mut p, c| {
        p.push(c);
        p
    })
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

    #[test]
    #[cfg(windows)]
    fn normalize_path_windows() {
        const CASES: [(&str, &str); 58] = [
            ("A//////././//.//B", r"A\B"),
            ("A/./B", r"A\B"),
            ("A/foo/../B", r"A\B"),
            ("C:A//B", r"C:A\B"),
            ("D:A/./B", r"D:A\B"),
            ("e:A/foo/../B", r"e:A\B"),
            ("C:///A//B", r"C:\A\B"),
            ("D:///A/./B", r"D:\A\B"),
            ("e:///A/foo/../B", r"e:\A\B"),
            ("..", r".."),
            (".", r"."),
            ("", r"."),
            ("/", "\\"),
            ("c:/", "c:\\"),
            ("/../.././..", "\\"),
            ("c:/../../..", "c:\\"),
            ("../.././..", r"..\..\.."),
            ("K:../.././..", r"K:..\..\.."),
            ("C:////a/b", r"C:\a\b"),
            ("//machine/share//a/b", r"\\machine\share\a\b"),
            // Node.js: r"\\.\NUL\"
            // Python: r"\\.\NUL"
            ("\\\\.\\NUL", r"\\.\NUL\"),
            // Node.js: r"\\?\D:\XY\Z"
            // Python: r"\\?\D:\XY\Z"
            ("\\\\?\\D:/XY\\Z", r"\\?\D:\XY\Z"),
            ("handbook/../../Tests/image.png", r"..\Tests\image.png"),
            (
                "handbook/../../../Tests/image.png",
                r"..\..\Tests\image.png",
            ),
            ("handbook///../a/.././../b/c", r"..\b\c"),
            ("handbook/a/../..///../../b/c", r"..\..\b\c"),
            ("//server/share/..", "\\\\server\\share\\"),
            ("//server/share/../", "\\\\server\\share\\"),
            ("//server/share/../..", "\\\\server\\share\\"),
            ("//server/share/../../", "\\\\server\\share\\"),
            // Node.js: "\\foo\\"
            // Python: "\\\\foo\\\\"
            ("\\\\foo\\\\", "\\foo"),
            // Node.js: "\\foo\\"
            // Python: "\\\\foo\\"
            ("\\\\foo\\", "\\foo"),
            // Node.js: "\\foo"
            // Python: "\\\\foo"
            ("\\\\foo", "\\foo"),
            // Node.js: "\\"
            // Python: "\\\\"
            ("\\\\", "\\"),
            ("./fixtures///b/../b/c.rs", "fixtures\\b\\c.rs"),
            ("/foo/../../../bar", "\\bar"),
            ("a//b//../b", "a\\b"),
            ("a//b//./c", "a\\b\\c"),
            ("a//b//.", "a\\b"),
            (
                "//server/share/dir/file.ext",
                "\\\\server\\share\\dir\\file.ext",
            ),
            ("/a/b/c/../../../x/y/z", "\\x\\y\\z"),
            // Node.js: "C:."
            // Python: "C:"
            ("C:", "C:"),
            ("C:..\\abc", "C:..\\abc"),
            ("C:..\\..\\abc\\..\\def", "C:..\\..\\def"),
            ("C:\\.", "C:\\"),
            ("file:stream", "file:stream"),
            // Node.js: "bar\\"
            // Python: "bar"
            ("bar\\foo..\\..\\", "bar"),
            ("bar\\foo..\\..", "bar"),
            ("bar\\foo..\\..\\baz", "bar\\baz"),
            // Node.js: "bar\\foo..\\"
            // Python: "bar\\foo.."
            ("bar\\foo..\\", "bar\\foo.."),
            ("bar\\foo..", "bar\\foo.."),
            ("..\\foo..\\..\\..\\bar", "..\\..\\bar"),
            ("..\\...\\..\\.\\...\\..\\..\\bar", "..\\..\\bar"),
            ("../../../foo/../../../bar", "..\\..\\..\\..\\..\\bar"),
            // Node.js: "..\\..\\..\\..\\..\\..\\"
            // Python: "..\\..\\..\\..\\..\\.."
            ("../../../foo/../../../bar/../../", "..\\..\\..\\..\\..\\.."),
            // Node.js: "..\\..\\"
            // Python: "..\\.."
            ("../foobar/barfoo/foo/../../../bar/../../", "..\\.."),
            (
                "../.../../foobar/../../../bar/../../baz",
                "..\\..\\..\\..\\baz",
            ),
            ("foo/bar\\baz", "foo\\bar\\baz"),
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
