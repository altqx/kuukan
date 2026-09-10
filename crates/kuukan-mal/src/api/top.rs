//! `api/top.rs`: the frozen top-list entry points.

use serde_json::Value;

use crate::client::MalClient;
use crate::error::MalError;
use crate::parser::top::{TopAnimeParser, TopCharactersParser, TopMangaParser, TopPeopleParser};
use crate::request::top::{
    TopAnimeRequest, TopCharactersRequest, TopMangaRequest, TopPeopleRequest,
};
use crate::request::MalRequest;

/// Wrap a parser failure like `ParserException::fromRequest()`.
fn parse_failed(path: &str, error: impl std::fmt::Display) -> MalError {
    MalError::parse_failed(path, error.to_string())
}

/// `MalClient::getTopAnime(TopAnimeRequest $request)`.
pub async fn get_top_anime(
    client: &MalClient,
    page: u64,
    r#type: Option<&str>,
) -> Result<Value, MalError> {
    let request = TopAnimeRequest::new(page, r#type)
        .map_err(|error| parse_failed("TopAnimeRequest", error))?;
    let path = request.path();
    let doc = client.get_html(&path).await?;
    TopAnimeParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getTopManga(TopMangaRequest $request)`.
pub async fn get_top_manga(
    client: &MalClient,
    page: u64,
    r#type: Option<&str>,
) -> Result<Value, MalError> {
    let request = TopMangaRequest::new(page, r#type)
        .map_err(|error| parse_failed("TopMangaRequest", error))?;
    let path = request.path();
    let doc = client.get_html(&path).await?;
    TopMangaParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getTopCharacters(TopCharactersRequest $request)`.
pub async fn get_top_characters(client: &MalClient, page: u64) -> Result<Value, MalError> {
    let path = TopCharactersRequest::new(page).path();
    let doc = client.get_html(&path).await?;
    TopCharactersParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}

/// `MalClient::getTopPeople(TopPeopleRequest $request)`.
pub async fn get_top_people(client: &MalClient, page: u64) -> Result<Value, MalError> {
    let path = TopPeopleRequest::new(page).path();
    let doc = client.get_html(&path).await?;
    TopPeopleParser::new(&doc)
        .get_model()
        .map_err(|error| parse_failed(&path, error))
}
