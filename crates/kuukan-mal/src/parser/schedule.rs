//! Port of `Jikan\Parser\Schedule\ScheduleParser`.
//!
//! Each day column holds anime cards parsed by the shared
//! [`crate::parser::common::anime_card`] parser.

use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::common::anime_card;
use crate::parser::helper::HtmlDoc;

/// `Jikan\Parser\Schedule\ScheduleParser`.
pub struct ScheduleParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> ScheduleParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        ScheduleParser { doc }
    }

    /// `Schedule::fromParser()`: one array per weekday plus `other`/`unknown`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "monday": self.get_shedule("monday")?,
            "tuesday": self.get_shedule("tuesday")?,
            "wednesday": self.get_shedule("wednesday")?,
            "thursday": self.get_shedule("thursday")?,
            "friday": self.get_shedule("friday")?,
            "saturday": self.get_shedule("saturday")?,
            "sunday": self.get_shedule("sunday")?,
            "other": self.get_shedule("other")?,
            "unknown": self.get_shedule("unknown")?,
        }))
    }

    /// `ScheduleParser::getShedule($day = 'all')`.
    pub fn get_shedule(&self, day: &str) -> Result<Vec<Value>, ParseError> {
        let mut parts: Vec<String> = vec!["/".to_string()];
        if day != "all" {
            parts.push(format!(
                "div[contains(@class, \"js-seasonal-anime-list-key-{day}\")]"
            ));
        }
        parts.push("div[contains(@class, \"seasonal-anime\")]".to_string());
        let query = parts.join("/");

        let mut out = Vec::new();
        for node in self.doc.nodes(&query)? {
            out.push(anime_card(&node)?);
        }
        Ok(out)
    }
}
