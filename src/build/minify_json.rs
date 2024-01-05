//! Minify JSON code.
//!
//! This module uses [`serde_json`] under the hood.

use super::{Entry, Error};

/// Minify JSON content of a [`Entry`].
///
/// This function minifies JSON code in the `content` property.
pub(super) fn minify_entry(entry: Entry) -> Result<Entry, Error> {
    if let Some(content) = entry.content.as_ref() {
        let content = minify(content).map_err(|error| Error::MinifyJson {
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

/// Minify a string containing JSON code.
pub(super) fn minify<S>(input: S) -> anyhow::Result<String>
where
    S: AsRef<str>,
{
    let input = input.as_ref();

    let data: serde_json::Value = serde_json::from_str(input)?;
    let output = serde_json::to_string(&data)?;

    Ok(output)
}

#[cfg(test)]
mod tests {
    #[test]
    fn minify() {
        const CASES: [(&str, &str); 1] = [(
            concat!(
                "{\n",                                                                  //
                "  \"glossary\": {\n",                                                  //
                "    \"title\": \"example glossary\",\n",                               //
                "    \"GlossDiv\": {\n",                                                //
                "      \"title\": \"S\",\n",                                            //
                "      \"GlossList\": {\n",                                             //
                "        \"GlossEntry\": {\n",                                          //
                "          \"ID\": \"SGML\",\n",                                        //
                "          \"SortAs\": \"SGML\",\n",                                    //
                "          \"GlossTerm\": \"Standard Generalized Markup Language\",\n", //
                "          \"Acronym\": \"SGML\",\n",                                   //
                "          \"Abbrev\": \"ISO 8879:1986\",\n",                           //
                "          \"GlossDef\": {\n",                                          //
                "            \"para\": \"A meta-markup language, used to create markup languages \
                 such as DocBook.\",\n", //
                "            \"GlossSeeAlso\": [\"GML\", \"XML\"]\n",                   //
                "          },\n",                                                       //
                "          \"GlossSee\": \"markup\"\n",                                 //
                "        }\n",                                                          //
                "      }\n",                                                            //
                "    }\n",                                                              //
                "  }\n",                                                                //
                "}\n"
            ),
            "{\"glossary\":{\"GlossDiv\":{\"GlossList\":{\"GlossEntry\":{\"Abbrev\":\"ISO \
             8879:1986\",\"Acronym\":\"SGML\",\"GlossDef\":{\"GlossSeeAlso\":[\"GML\",\"XML\"],\"\
             para\":\"A meta-markup language, used to create markup languages such as \
             DocBook.\"},\"GlossSee\":\"markup\",\"GlossTerm\":\"Standard Generalized Markup \
             Language\",\"ID\":\"SGML\",\"SortAs\":\"SGML\"}},\"title\":\"S\"},\"title\":\"\
             example glossary\"}}",
        )];

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
