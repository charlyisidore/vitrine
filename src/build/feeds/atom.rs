//! Utility structures for Atom feeds.
//!
//! The structures follow the [RFC 4287](https://www.rfc-editor.org/rfc/rfc4287) specification.

use serde::Serialize;

/// XML namespace for Atom feeds.
pub const XMLNS: &str = "http://www.w3.org/2005/Atom";

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
pub struct CommonAttributes {
    /// Base URI (or IRI) for resolving any relative references found within the
    /// effective scope of the xml:base attribute.
    #[serde(rename = "@xml:base", skip_serializing_if = "Option::is_none")]
    pub base: Option<Uri>,
    /// Natural language for the element and its descendents.
    #[serde(rename = "@xml:lang", skip_serializing_if = "Option::is_none")]
    pub lang: Option<LanguageTag>,
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
pub struct Category {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// String that identifies the category to which the entry or feed belongs.
    pub term: Text,
    /// IRI that identifies a categorization scheme.
    #[serde(rename = "@scheme", skip_serializing_if = "Option::is_none")]
    pub scheme: Option<Uri>,
    /// Human-readable label for display in end-user applications.
    #[serde(rename = "@label", skip_serializing_if = "Option::is_none")]
    pub label: Option<Text>,
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
pub struct Content {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// Content type.
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<MediaType>,
    /// IRI reference.
    #[serde(rename = "@src", skip_serializing_if = "Option::is_none")]
    pub src: Option<Uri>,
    /// Content value.
    #[serde(rename = "$value", skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
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
pub struct Entry {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// Author of the entry or feed.
    pub author: Vec<Author>,
    /// Information about a category associated with an entry or feed.
    pub category: Vec<Category>,
    /// Either contains or links to the content of the entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Content>,
    /// Person or other entity who contributed to the entry or feed.
    pub contributor: Vec<Contributor>,
    /// Permanent, universally unique identifier for an entry or feed.
    pub id: Id,
    /// Reference from an entry or feed to a Web resource.
    pub link: Vec<Link>,
    /// Instant in time associated with an event early in the life cycle of the
    /// entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published: Option<Published>,
    /// Information about rights held in and over an entry or feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights: Option<Rights>,
    /// Source feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Short summary, abstract, or excerpt of an entry.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<Summary>,
    /// Human-readable title for an entry or feed.
    pub title: Title,
    /// The most recent instant in time when an entry or feed was modified in a
    /// way the publisher considers significant.
    pub updated: Updated,
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
pub struct Feed {
    /// XML namespace.
    #[serde(rename = "@xmlns")]
    pub xmlns: &'static str,
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// Author of the entry or feed.
    pub author: Vec<Author>,
    /// Information about a category associated with an entry or feed.
    pub category: Vec<Category>,
    /// Person or other entity who contributed to the entry or feed.
    pub contributor: Vec<Contributor>,
    /// Agent used to generate a feed, for debugging and other purposes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generator: Option<Generator>,
    /// IRI reference that identifies an image that provides iconic visual
    /// identification for a feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    /// Permanent, universally unique identifier for an entry or feed.
    pub id: Id,
    /// Reference from an entry or feed to a Web resource.
    pub link: Vec<Link>,
    /// IRI reference that identifies an image that provides visual
    /// identification for a feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Logo>,
    /// Information about rights held in and over an entry or feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights: Option<Rights>,
    /// Human-readable description or subtitle for a feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<Subtitle>,
    /// Human-readable title for an entry or feed.
    pub title: Title,
    /// The most recent instant in time when an entry or feed was modified in a
    /// way the publisher considers significant.
    pub updated: Updated,
    /// Individual entry, acting as a container for metadata and data associated
    /// with the entry.
    pub entry: Vec<Entry>,
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
pub struct Generator {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// IRI reference.
    #[serde(rename = "@uri", skip_serializing_if = "Option::is_none")]
    pub uri: Option<Uri>,
    /// Version of the generating agent.
    #[serde(rename = "@version", skip_serializing_if = "Option::is_none")]
    pub version: Option<Text>,
    /// Human-readable name for the generating agent.
    #[serde(rename = "$text")]
    pub text: Text,
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
pub struct Link {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// Link's IRI.
    #[serde(rename = "@href")]
    pub href: Uri,
    /// Link relation type.
    #[serde(rename = "@rel", skip_serializing_if = "Option::is_none")]
    pub rel: Option<String>,
    /// Advisory media type.
    #[serde(rename = "@type", skip_serializing_if = "Option::is_none")]
    pub r#type: Option<MediaType>,
    /// Language of the resource pointed to by the href attribute.
    #[serde(rename = "@hreflang", skip_serializing_if = "Option::is_none")]
    pub hreflang: Option<LanguageTag>,
    /// Human-readable information about the link.
    #[serde(rename = "@title", skip_serializing_if = "Option::is_none")]
    pub title: Option<Text>,
    /// Advisory length of the linked content in octets.
    #[serde(rename = "@length", skip_serializing_if = "Option::is_none")]
    pub length: Option<Text>,
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
pub struct PersonConstruct {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// Human-readable name for the person.
    pub name: Text,
    /// IRI associated with the person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uri: Option<Uri>,
    /// E-mail address associated with the person.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<EmailAddress>,
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
pub struct Source {
    /// `xml:*` attributes.
    #[serde(flatten)]
    pub xml: CommonAttributes,
    /// Author of the entry or feed.
    pub author: Vec<Author>,
    /// Information about a category associated with an entry or feed.
    pub category: Vec<Category>,
    /// Person or other entity who contributed to the entry or feed.
    pub contributor: Vec<Contributor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    /// Agent used to generate a feed, for debugging and other purposes.
    pub generator: Option<Generator>,
    /// IRI reference that identifies an image that provides iconic visual
    /// identification for a feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<Icon>,
    /// Permanent, universally unique identifier for an entry or feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Id>,
    /// Reference from an entry or feed to a Web resource.
    pub link: Vec<Link>,
    /// IRI reference that identifies an image that provides visual
    /// identification for a feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo: Option<Logo>,
    /// Information about rights held in and over an entry or feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rights: Option<Rights>,
    /// Human-readable description or subtitle for a feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<Subtitle>,
    /// Human-readable title for an entry or feed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<Title>,
    /// The most recent instant in time when an entry or feed was modified in a
    /// way the publisher considers significant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<Updated>,
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
