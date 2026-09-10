//! Port of `Jikan\Parser\SeasonList\*`.

use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::helper::{HtmlDoc, HtmlNode};

/// `Constants::SEASONS` (`['Winter', 'Spring', 'Summer', 'Fall']`).
pub const SEASONS: [&str; 4] = ["Winter", "Spring", "Summer", "Fall"];

/// `Jikan\Parser\SeasonList\SeasonListParser`.
pub struct SeasonListParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> SeasonListParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        SeasonListParser { doc }
    }

    /// `SeasonArchive::fromParser()`: `{results}`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({ "results": self.get_results()? }))
    }

    /// `SeasonListParser::getResults()`.
    pub fn get_results(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self
            .doc
            .nodes("//table[contains(@class, \"anime-seasonal-byseason\")]//tr")?
        {
            out.push(SeasonListItemParser::new(&node).get_model()?);
        }
        Ok(out)
    }
}

/// `Jikan\Parser\SeasonList\SeasonListItemParser`.
pub struct SeasonListItemParser<'a> {
    node: &'a HtmlNode,
}

impl<'a> SeasonListItemParser<'a> {
    pub fn new(node: &'a HtmlNode) -> Self {
        SeasonListItemParser { node }
    }

    /// `SeasonListItem::fromParser()`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "year": self.get_year()?,
            "seasons": self.get_seasons()?,
        }))
    }

    /// `SeasonListItemParser::getYear()`: `(int) preg_replace('/\D/', '', first td)`.
    pub fn get_year(&self) -> Result<i64, ParseError> {
        let text = self.node.text("//td")?.unwrap_or_default();
        let digits: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
        Ok(digits.parse().unwrap_or(0))
    }

    /// `SeasonListItemParser::getSeasons()`: `array_filter(Constants::SEASONS)`
    /// on the whole row text, then `array_map('strtolower')`.
    pub fn get_seasons(&self) -> Result<Vec<String>, ParseError> {
        let text = self.node.node_text();
        // `array_filter` preserves the original keys; only sequential results
        // serialize as a JSON array, so non-matching keys are skipped here.
        let mut out = Vec::new();
        for season in SEASONS {
            if text.contains(season) {
                out.push(season.to_ascii_lowercase());
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::helper::HtmlDoc;

    #[test]
    fn seasons_are_filtered_and_lowercased() {
        let doc = HtmlDoc::parse_str(
            r#"
            <table class="anime-seasonal-byseason">
              <tr><td>2023</td><td>Winter Spring</td></tr>
              <tr><td>2022</td><td>Winter Spring Summer Fall</td></tr>
            </table>
            "#,
        )
        .unwrap();
        let model = SeasonListParser::new(&doc).get_model().unwrap();
        assert_eq!(model["results"][0]["year"], 2023);
        assert_eq!(model["results"][0]["seasons"], json!(["winter", "spring"]));
        assert_eq!(model["results"][1]["seasons"], json!(["winter", "spring", "summer", "fall"]));
    }
}
