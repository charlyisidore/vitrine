//! Minify XML code.
//!
//! This module uses [`quick_xml`] under the hood.

use quick_xml::{
    events::{attributes::Attribute, BytesStart, Event},
    Reader, Writer,
};

use super::{Entry, Error};

/// Minify XML content of a [`Entry`].
///
/// This function minifies XML code in the `content` property.
pub(super) fn minify_entry(entry: Entry) -> Result<Entry, Error> {
    if let Some(content) = entry.content.as_ref() {
        let content = minify(content).map_err(|error| Error::MinifyXml {
            input_path: entry.input_path_buf(),
            source: error,
        })?;

        return Ok(Entry {
            content: Some(content),
            ..entry
        });
    }

    Ok(entry)
}

/// Minify a string containing XML code.
pub(super) fn minify<S>(input: S) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let input = input.as_ref();

    let mut reader = Reader::from_str(input);

    reader.trim_text(true);

    let mut writer = Writer::new(Vec::new());

    loop {
        let event = reader.read_event()?;

        match event {
            Event::Eof => break,
            Event::Start(bytes) => {
                let bytes = minify_bytes_start(bytes)?;
                writer.write_event(Event::Start(bytes))?
            },
            Event::Empty(bytes) => {
                let bytes = minify_bytes_start(bytes)?;
                writer.write_event(Event::Empty(bytes))?
            },
            _ => writer.write_event(event)?,
        }
    }

    let output = String::from_utf8(writer.into_inner())?;

    Ok(output)
}

/// Remove unncessary spaces between XML attributes.
fn minify_bytes_start(bytes: BytesStart) -> anyhow::Result<BytesStart> {
    let name = std::str::from_utf8(bytes.name().into_inner())?.to_owned();
    let attributes: Vec<Attribute> = bytes.attributes().collect::<Result<_, _>>()?;
    Ok(BytesStart::new(name).with_attributes(attributes))
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify() {
        const CASES: [(&str, &str); 4] = [
            (
                concat!(
                    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n", //
                    "<urlset>\n",                                   //
                    "  <url>\n",                                    //
                    "    <loc>http://www.example.com/</loc>\n",     //
                    "    <lastmod>2005-01-01</lastmod>\n",          //
                    "    <changefreq>monthly</changefreq>\n",       //
                    "    <priority>0.8</priority>\n",               //
                    "  </url>\n",                                   //
                    "</urlset>\n"
                ),
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?><urlset><url><loc>http://www.example.com\
                 /</loc><lastmod>2005-01-01</lastmod><changefreq>monthly</changefreq><priority>0.8<\
                 /priority></url></urlset>",
            ),
            (
                concat!(
                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n", //
                    "<feed xmlns=\"http://www.w3.org/2005/Atom\">\n", //
                    "  <title>Example Feed</title>\n", //
                    "  <link href=\"http://example.org/\"/>\n", //
                    "  <updated>2003-12-13T18:30:02Z</updated>\n", //
                    "  <author>\n", //
                    "    <name>John Doe</name>\n", //
                    "  </author>\n", //
                    "  <id>urn:uuid:60a76c80-d399-11d9-b93C-0003939e0af6</id>\n", //
                    "  <entry>\n", //
                    "    <title>Atom-Powered Robots Run Amok</title>\n", //
                    "    <link href=\"http://example.org/2003/12/13/atom03\"/>\n", //
                    "    <id>urn:uuid:1225c695-cfb8-4ebb-aaaa-80da344efa6a</id>\n", //
                    "    <updated>2003-12-13T18:30:02Z</updated>\n",//
                    "    <summary>Some text.</summary>\n", //
                    "  </entry>\n",//
                    "</feed>\n"
                ),
                "<?xml version=\"1.0\" encoding=\"utf-8\"?><feed xmlns=\"http://www.w3.org/2005/Ato\
                 m\"><title>Example Feed</title><link href=\"http://example.org/\"/><updated>2003-1\
                 2-13T18:30:02Z</updated><author><name>John Doe</name></author><id>urn:uuid:60a76c8\
                 0-d399-11d9-b93C-0003939e0af6</id><entry><title>Atom-Powered Robots Run Amok</titl\
                 e><link href=\"http://example.org/2003/12/13/atom03\"/><id>urn:uuid:1225c695-cfb8-\
                 4ebb-aaaa-80da344efa6a</id><updated>2003-12-13T18:30:02Z</updated><summary>Some te\
                 xt.</summary></entry></feed>"
            ),
            (
                concat!(
                    "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n",
                    "<feed xmlns=\"http://www.w3.org/2005/Atom\">\n",
                    "  <title type=\"text\">dive into mark</title>\n",
                    "  <subtitle type=\"html\">\n",
                    "    A &lt;em&gt;lot&lt;/em&gt; of effort\n",
                    "    went into making this effortless\n",
                    "  </subtitle>\n",
                    "  <updated>2005-07-31T12:29:29Z</updated>\n",
                    "  <id>tag:example.org,2003:3</id>\n",
                    "  <link rel=\"alternate\" type=\"text/html\"\n",
                    "   hreflang=\"en\" href=\"http://example.org/\"/>\n",
                    "  <link rel=\"self\" type=\"application/atom+xml\"\n",
                    "   href=\"http://example.org/feed.atom\"/>\n",
                    "  <rights>Copyright (c) 2003, Mark Pilgrim</rights>\n",
                    "  <generator uri=\"http://www.example.com/\" version=\"1.0\">\n",
                    "    Example Toolkit\n",
                    "  </generator>\n",
                    "  <entry>\n",
                    "    <title>Atom draft-07 snapshot</title>\n",
                    "    <link rel=\"alternate\" type=\"text/html\"\n",
                    "     href=\"http://example.org/2005/04/02/atom\"/>\n",
                    "    <link rel=\"enclosure\" type=\"audio/mpeg\" length=\"1337\"\n",
                    "     href=\"http://example.org/audio/ph34r_my_podcast.mp3\"/>\n",
                    "    <id>tag:example.org,2003:3.2397</id>\n",
                    "    <updated>2005-07-31T12:29:29Z</updated>\n",
                    "    <published>2003-12-13T08:29:29-04:00</published>\n",
                    "    <author>\n",
                    "      <name>Mark Pilgrim</name>\n",
                    "      <uri>http://example.org/</uri>\n",
                    "      <email>f8dy@example.com</email>\n",
                    "    </author>\n",
                    "    <contributor>\n",
                    "      <name>Sam Ruby</name>\n",
                    "    </contributor>\n",
                    "    <contributor>\n",
                    "      <name>Joe Gregorio</name>\n",
                    "    </contributor>\n",
                    "    <content type=\"xhtml\" xml:lang=\"en\"\n",
                    "     xml:base=\"http://diveintomark.org/\">\n",
                    "      <div xmlns=\"http://www.w3.org/1999/xhtml\">\n",
                    "        <p><i>[Update: The Atom draft is finished.]</i></p>\n",
                    "      </div>\n",
                    "    </content>\n",
                    "  </entry>\n",
                    "</feed>\n"
                ),
                "<?xml version=\"1.0\" encoding=\"utf-8\"?><feed xmlns=\"http://www.w3.org/2005/Ato\
                 m\"><title type=\"text\">dive into mark</title><subtitle type=\"html\">A &lt;em&gt\
                 ;lot&lt;/em&gt; of effort\n    went into making this effortless</subtitle><updated\
                 >2005-07-31T12:29:29Z</updated><id>tag:example.org,2003:3</id><link rel=\"alternat\
                 e\" type=\"text/html\" hreflang=\"en\" href=\"http://example.org/\"/><link rel=\"s\
                 elf\" type=\"application/atom+xml\" href=\"http://example.org/feed.atom\"/><rights\
                 >Copyright (c) 2003, Mark Pilgrim</rights><generator uri=\"http://www.example.com/\
                 \" version=\"1.0\">Example Toolkit</generator><entry><title>Atom draft-07 snapshot\
                 </title><link rel=\"alternate\" type=\"text/html\" href=\"http://example.org/2005/\
                 04/02/atom\"/><link rel=\"enclosure\" type=\"audio/mpeg\" length=\"1337\" href=\"h\
                 ttp://example.org/audio/ph34r_my_podcast.mp3\"/><id>tag:example.org,2003:3.2397</i\
                 d><updated>2005-07-31T12:29:29Z</updated><published>2003-12-13T08:29:29-04:00</pub\
                 lished><author><name>Mark Pilgrim</name><uri>http://example.org/</uri><email>f8dy@\
                 example.com</email></author><contributor><name>Sam Ruby</name></contributor><contr\
                 ibutor><name>Joe Gregorio</name></contributor><content type=\"xhtml\" xml:lang=\"e\
                 n\" xml:base=\"http://diveintomark.org/\"><div xmlns=\"http://www.w3.org/1999/xhtm\
                 l\"><p><i>[Update: The Atom draft is finished.]</i></p></div></content></entry></f\
                 eed>"
            ),
            (
                concat!(
                    "<script type=\"text/javascript\">\n",
                    "  //<![CDATA[\n",
                    "  document.write(\"<\");\n",
                    "  //]]>\n",
                    "</script>\n"
                ),
                "<script type=\"text/javascript\">//<![CDATA[\n  document.write(\"<\");\n  //]]></s\
                 cript>"
            ),
        ];

        for (input, expected) in CASES {
            let result = super::minify(input).unwrap();
            assert_eq!(
                result,
                expected.to_owned(),
                "\nminify({input:?}) expected {expected:?} but received {result:?}"
            );
        }
    }
}
