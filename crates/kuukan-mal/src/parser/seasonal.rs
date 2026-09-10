//! Port of `Jikan\Parser\Seasonal\SeasonalParser`.
//!
//! Anime cards come from the shared [`crate::parser::common::anime_card`]
//! parser; seasonal cards append the `continuing` flag
//! (`Jikan\Model\Seasonal\SeasonalAnime`).

use serde_json::{json, Value};

use crate::error::ParseError;
use crate::parser::common::{anime_card, anime_card_continuing};
use crate::parser::helper::{HtmlDoc, HtmlNode};
use crate::parser::jstring::cleanse;

/// `Jikan\Parser\Seasonal\SeasonalParser`.
pub struct SeasonalParser<'a> {
    doc: &'a HtmlDoc,
}

impl<'a> SeasonalParser<'a> {
    pub fn new(doc: &'a HtmlDoc) -> Self {
        SeasonalParser { doc }
    }

    /// `Seasonal::fromParser()`: `{season_name, season_year, anime}`.
    pub fn get_model(&self) -> Result<Value, ParseError> {
        Ok(json!({
            "season_name": self.get_season_name()?,
            "season_year": self.get_season_year()?,
            "anime": self.get_seasonal_anime()?,
        }))
    }

    /// `SeasonalParser::getSeasonalAnime()`.
    pub fn get_seasonal_anime(&self) -> Result<Vec<Value>, ParseError> {
        let mut out = Vec::new();
        for node in self.doc.css_nodes("div.seasonal-anime.js-seasonal-anime")? {
            let mut card = anime_card(&node)?;
            let continuing = anime_card_continuing(&node);
            if let Value::Object(map) = &mut card {
                map.insert("continuing".to_string(), Value::Bool(continuing));
            }
            patch_month_year_airing_start(&mut card, &node)?;
            out.push(card);
        }
        Ok(out)
    }

    /// `SeasonalParser::getSeasonName()`.
    pub fn get_season_name(&self) -> Result<Option<String>, ParseError> {
        let Some(season) = self.season_parts()? else {
            return Ok(None);
        };
        Ok(season.first().cloned())
    }

    /// `SeasonalParser::getSeasonYear()`.
    pub fn get_season_year(&self) -> Result<Option<i64>, ParseError> {
        let Some(season) = self.season_parts()? else {
            return Ok(None);
        };
        let Some(year) = season.get(1) else {
            return Ok(None);
        };
        Ok(year.parse::<i64>().ok())
    }

    /// `explode(' ', JString::cleanse($node->text()))` for `div.navi-seasonal a.on`.
    fn season_parts(&self) -> Result<Option<Vec<String>>, ParseError> {
        let Some(node) = self.doc.css_nodes("div.navi-seasonal a.on")?.into_iter().next() else {
            return Ok(None);
        };
        Ok(Some(
            cleanse(&node.node_text())
                .split(' ')
                .map(|part| part.to_string())
                .collect(),
        ))
    }
}

/// TODO(parser-common): remove when `common.rs::parse_jst_datetime()` handles
/// month-only air dates.
///
/// `AnimeCardParser::getAirDates()` uses PHP's `new DateTimeImmutable($date,
/// new DateTimeZone('JST'))`; MAL sometimes renders the date as `Apr 2018`
/// (no day), which PHP normalizes to 2018-04-01 00:00 JST. The shared card
/// parser currently returns `null` for that format, so patch the one field
/// here while keeping the payload identical to PHP.
fn patch_month_year_airing_start(card: &mut Value, node: &HtmlNode) -> Result<(), ParseError> {
    let already_set = card
        .get("airing_start")
        .map(|value| !value.is_null())
        .unwrap_or(false);
    if already_set {
        return Ok(());
    }
    let Some(dates) = node.first("//div/div[2]/div[2]/span[contains(@class, \"item\")][1]")? else {
        return Ok(());
    };
    let text = cleanse(&dates.node_text()).replace("(JST)", "");
    let Some(date) = parse_month_year_jst(&text) else {
        return Ok(());
    };
    if let Value::Object(map) = card {
        map.insert(
            "airing_start".to_string(),
            Value::String(crate::parser::date::format_atom(
                &date.with_timezone(&chrono::Utc),
            )),
        );
    }
    Ok(())
}

/// `new DateTimeImmutable('Apr 2018', new DateTimeZone('JST'))`.
fn parse_month_year_jst(text: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    let caps = month_year_re().captures(text.trim())?;
    let month = match caps[1].to_ascii_lowercase().as_str() {
        "jan" | "january" => 1,
        "feb" | "february" => 2,
        "mar" | "march" => 3,
        "apr" | "april" => 4,
        "may" => 5,
        "jun" | "june" => 6,
        "jul" | "july" => 7,
        "aug" | "august" => 8,
        "sep" | "september" => 9,
        "oct" | "october" => 10,
        "nov" | "november" => 11,
        "dec" | "december" => 12,
        _ => return None,
    };
    let year: i32 = caps[2].parse().ok()?;
    let naive = chrono::NaiveDate::from_ymd_opt(year, month, 1)?.and_hms_opt(0, 0, 0)?;
    let jst = chrono::FixedOffset::east_opt(9 * 3600)?;
    chrono::TimeZone::from_local_datetime(&jst, &naive).single()
}

fn month_year_re() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"^([A-Za-z]+)\s+(\d{4})$").expect("valid regex"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::helper::HtmlDoc;

    #[test]
    fn season_name_and_year() {
        let doc = HtmlDoc::parse_str(
            r#"
            <div class="navi-seasonal">
              <a href="/anime/season/2000/fall">Fall 2000</a>
              <a class="on" href="/anime/season/2018/spring">Spring 2018</a>
            </div>
            "#,
        )
        .unwrap();
        let parser = SeasonalParser::new(&doc);
        assert_eq!(parser.get_season_name().unwrap().as_deref(), Some("Spring"));
        assert_eq!(parser.get_season_year().unwrap(), Some(2018));
    }

    #[test]
    fn missing_season_is_null() {
        let doc = HtmlDoc::parse_str("<div></div>").unwrap();
        let parser = SeasonalParser::new(&doc);
        assert_eq!(parser.get_season_name().unwrap(), None);
        assert_eq!(parser.get_season_year().unwrap(), None);
    }

    #[test]
    fn month_year_jst_matches_php() {
        // `new DateTimeImmutable('Apr 2018', new DateTimeZone('JST'))`
        // -> 2018-03-31T15:00:00+00:00.
        let date = parse_month_year_jst("Apr 2018").unwrap();
        assert_eq!(
            crate::parser::date::format_atom(&date.with_timezone(&chrono::Utc)),
            "2018-03-31T15:00:00+00:00"
        );
        assert!(parse_month_year_jst("2018").is_none());
        assert!(parse_month_year_jst("Nope 2018").is_none());
    }
}
