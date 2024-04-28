//! Parse and manipulate the query component in URLs.

/// An URL query component.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
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
            ("name=ferret&color=purple", &[
                ("name", Some("ferret")),
                ("color", Some("purple")),
            ]),
            ("objectClass?one", &[("objectClass?one", None)]),
        ];

        for (input, expected) in CASES {
            let query = Query::from(input);
            let result: Vec<_> = query.parameters().collect();
            assert_eq!(result, expected, "{input:?}");
        }
    }
}
