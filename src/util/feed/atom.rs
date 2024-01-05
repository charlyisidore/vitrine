//! Utility structures for Atom feeds.
//!
//! The structures follow the [RFC 4287](https://www.rfc-editor.org/rfc/rfc4287) specification.

use serde::Serialize;

/// XML namespace for Atom feeds.
pub(crate) const XMLNS: &str = "http://www.w3.org/2005/Atom";

/// Author.
///
/// ```text
/// atomAuthor = element atom:author { atomPersonConstruct }
/// ```
type Author = PersonConstruct;

/// Common attributes.
///
/// ```text
/// atomCommonAttributes =
///    attribute xml:base { atomUri }?,
///    attribute xml:lang { atomLanguageTag }?,
///    undefinedAttribute*
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct CommonAttributes {
    #[serde(rename = "@xml:base", skip_serializing_if = "Option::is_none")]
    pub(crate) base: Option<Uri>,
    #[serde(rename = "@xml:lang", skip_serializing_if = "Option::is_none")]
    pub(crate) lang: Option<LanguageTag>,
}

/// Category.
///
/// ```text
/// atomCategory =
///    element atom:category {
///       atomCommonAttributes,
///       attribute term { text },
///       attribute scheme { atomUri }?,
///       attribute label { text }?,
///       undefinedContent
///    }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Category {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    pub(crate) term: Text,
    #[serde(rename = "@scheme", skip_serializing_if = "Option::is_none")]
    pub(crate) scheme: Option<Uri>,
    #[serde(rename = "@label", skip_serializing_if = "Option::is_none")]
    pub(crate) label: Option<Text>,
}

/// Content.
///
/// ```text
/// atomContent = atomInlineTextContent
///  | atomInlineXHTMLContent
///  | atomInlineOtherContent
///  | atomOutOfLineContent
///
/// atomInlineTextContent =
///    element atom:content {
///       atomCommonAttributes,
///       attribute type { "text" | "html" }?,
///       (text)*
///    }
///
/// atomInlineXHTMLContent =
///    element atom:content {
///       atomCommonAttributes,
///       attribute type { "xhtml" },
///       xhtmlDiv
///    }
///
/// atomInlineOtherContent =
///    element atom:content {
///       atomCommonAttributes,
///       attribute type { atomMediaType }?,
///       (text|anyElement)*
///    }
///
/// atomOutOfLineContent =
///    element atom:content {
///       atomCommonAttributes,
///       attribute type { atomMediaType }?,
///       attribute src { atomUri },
///       empty
///    }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Content {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    pub(crate) r#type: Option<MediaType>,
    #[serde(rename = "@src", skip_serializing_if = "Option::is_none")]
    pub(crate) src: Option<Uri>,
    #[serde(rename = "$value", skip_serializing_if = "Option::is_none")]
    pub(crate) value: Option<String>,
}

/// Contributor.
///
/// ```text
/// atomContributor = element atom:contributor { atomPersonConstruct }
/// ```
type Contributor = PersonConstruct;

/// Date construct.
///
/// ```text
/// atomDateConstruct =
///    atomCommonAttributes,
///    xsd:dateTime
/// ```
type DateConstruct = String;

/// Email address.
///
/// ```text
/// atomEmailAddress = xsd:string { pattern = ".+@.+" }
/// ```
type EmailAddress = String;

/// Entry.
///
/// ```text
/// atomEntry =
///    element atom:entry {
///       atomCommonAttributes,
///       (atomAuthor*
///        & atomCategory*
///        & atomContent?
///        & atomContributor*
///        & atomId
///        & atomLink*
///        & atomPublished?
///        & atomRights?
///        & atomSource?
///        & atomSummary?
///        & atomTitle
///        & atomUpdated
///        & extensionElement*)
///    }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Entry {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    pub(crate) author: Vec<Author>,
    pub(crate) category: Vec<Category>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<Content>,
    pub(crate) contributor: Vec<Contributor>,
    pub(crate) id: Id,
    pub(crate) link: Vec<Link>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) published: Option<Published>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rights: Option<Rights>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<Source>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) summary: Option<Summary>,
    pub(crate) title: Title,
    pub(crate) updated: Updated,
}

/// Feed.
///
/// ```text
/// atomFeed =
///    element atom:feed {
///       atomCommonAttributes,
///       (atomAuthor*
///        & atomCategory*
///        & atomContributor*
///        & atomGenerator?
///        & atomIcon?
///        & atomId
///        & atomLink*
///        & atomLogo?
///        & atomRights?
///        & atomSubtitle?
///        & atomTitle
///        & atomUpdated
///        & extensionElement*),
///       atomEntry*
///    }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Feed {
    #[serde(rename = "@xmlns")]
    pub(crate) xmlns: &'static str,
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    pub(crate) author: Vec<Author>,
    pub(crate) category: Vec<Category>,
    pub(crate) contributor: Vec<Contributor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) generator: Option<Generator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<Icon>,
    pub(crate) id: Id,
    pub(crate) link: Vec<Link>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) logo: Option<Logo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rights: Option<Rights>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) subtitle: Option<Subtitle>,
    pub(crate) title: Title,
    pub(crate) updated: Updated,
    pub(crate) entry: Vec<Entry>,
}

/// Generator.
///
/// ```text
/// atomGenerator = element atom:generator {
///    atomCommonAttributes,
///    attribute uri { atomUri }?,
///    attribute version { text }?,
///    text
/// }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Generator {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    #[serde(rename = "@uri", skip_serializing_if = "Option::is_none")]
    pub(crate) uri: Option<Uri>,
    #[serde(rename = "@version", skip_serializing_if = "Option::is_none")]
    pub(crate) version: Option<Text>,
    #[serde(rename = "$text")]
    pub(crate) text: Text,
}

/// Icon.
///
/// ```text
/// atomIcon = element atom:icon {
///    atomCommonAttributes,
///    (atomUri)
/// }
/// ```
type Icon = Uri;

/// Id.
///
/// ```text
/// atomId = element atom:id {
///    atomCommonAttributes,
///    (atomUri)
/// }
/// ```
type Id = Uri;

/// Language tag.
///
/// As defined in [RFC 3066](https://www.rfc-editor.org/rfc/rfc3066).
///
/// ```text
/// atomLanguageTag = xsd:string {
///    pattern = "[A-Za-z]{1,8}(-[A-Za-z0-9]{1,8})*"
/// }
/// ```
type LanguageTag = String;

/// Link.
///
/// ```text
/// atomLink =
///    element atom:link {
///       atomCommonAttributes,
///       attribute href { atomUri },
///       attribute rel { atomNCName | atomUri }?,
///       attribute type { atomMediaType }?,
///       attribute hreflang { atomLanguageTag }?,
///       attribute title { text }?,
///       attribute length { text }?,
///       undefinedContent
///   }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Link {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    #[serde(rename = "@href")]
    pub(crate) href: Uri,
    #[serde(rename = "@rel", skip_serializing_if = "Option::is_none")]
    pub(crate) rel: Option<String>,
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    pub(crate) r#type: Option<MediaType>,
    #[serde(rename = "@hreflang", skip_serializing_if = "Option::is_none")]
    pub(crate) hreflang: Option<LanguageTag>,
    #[serde(rename = "@title", skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<Text>,
    #[serde(rename = "@length", skip_serializing_if = "Option::is_none")]
    pub(crate) length: Option<Text>,
}

/// Logo.
///
/// ```text
/// atomLogo = element atom:logo {
///    atomCommonAttributes,
///    (atomUri)
/// }
/// ```
type Logo = Uri;

/// Media type.
///
/// ```text
/// atomMediaType = xsd:string { pattern = ".+/.+" }
/// ```
type MediaType = String;

/// Person construct.
///
/// ```text
/// atomPersonConstruct =
///    atomCommonAttributes,
///    (element atom:name { text }
///     & element atom:uri { atomUri }?
///     & element atom:email { atomEmailAddress }?
///     & extensionElement*)
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct PersonConstruct {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    pub(crate) name: Text,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) uri: Option<Uri>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) email: Option<EmailAddress>,
}

/// Published.
///
/// ```text
/// atomPublished = element atom:published { atomDateConstruct }
/// ```
type Published = DateConstruct;

/// Rights.
///
/// ```text
/// atomRights = element atom:rights { atomTextConstruct }
/// ```
type Rights = TextConstruct;

/// Source.
///
/// ```text
/// atomSource =
///    element atom:source {
///       atomCommonAttributes,
///       (atomAuthor*
///        & atomCategory*
///        & atomContributor*
///        & atomGenerator?
///        & atomIcon?
///        & atomId?
///        & atomLink*
///        & atomLogo?
///        & atomRights?
///        & atomSubtitle?
///        & atomTitle?
///        & atomUpdated?
///        & extensionElement*)
///    }
/// ```
#[derive(Debug, Default, Serialize)]
pub(crate) struct Source {
    #[serde(flatten)]
    pub(crate) xml: CommonAttributes,
    pub(crate) author: Vec<Author>,
    pub(crate) category: Vec<Category>,
    pub(crate) contributor: Vec<Contributor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) generator: Option<Generator>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) icon: Option<Icon>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) id: Option<Id>,
    pub(crate) link: Vec<Link>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) logo: Option<Logo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) rights: Option<Rights>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) subtitle: Option<Subtitle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) title: Option<Title>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) updated: Option<Updated>,
}

/// Subtitle.
///
/// ```text
/// atomSubtitle = element atom:subtitle { atomTextConstruct }
/// ```
type Subtitle = TextConstruct;

/// Summary.
///
/// ```text
/// atomSummary = element atom:summary { atomTextConstruct }
/// ```
type Summary = TextConstruct;

/// Text.
type Text = String;

/// Text construct.
///
/// ```text
/// atomPlainTextConstruct =
///    atomCommonAttributes,
///    attribute type { "text" | "html" }?,
///    text
///
/// atomXHTMLTextConstruct =
///    atomCommonAttributes,
///    attribute type { "xhtml" },
///    xhtmlDiv
///
/// atomTextConstruct = atomPlainTextConstruct | atomXHTMLTextConstruct
/// ```
type TextConstruct = Text;

/// Title.
///
/// ```text
/// atomTitle = element atom:title { atomTextConstruct }
/// ```
type Title = TextConstruct;

/// Updated.
///
/// ```text
/// atomUpdated = element atom:updated { atomDateConstruct }
/// ```
type Updated = DateConstruct;

/// URI.
///
/// ```text
/// atomUri = text
/// ```
type Uri = Text;
