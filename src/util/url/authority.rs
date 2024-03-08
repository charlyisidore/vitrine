//! Parse and manipulate the authority component in URLs.

/// An URL authority component.
#[derive(Clone, Debug, Default, Hash, PartialEq)]
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
                },
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
                },
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

#[cfg(test)]
mod tests {
    use super::{Authority, Component};

    #[test]
    fn components() {
        const CASES: [(&str, &[Component]); 8] = [
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
            ("example.com:", &[
                Component::Host("example.com"),
                Component::Port(""),
            ]),
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
