//! Frozen producer entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::producer::{ProducerListParser, ProducerParser};
use crate::request::producer::{ProducerRequest, ProducersRequest};
use crate::request::MalRequest;
use crate::source::{MalSource, MalSourceExt};

/// `MalClient::getProducer()` — `/anime/producer/{id}`.
pub async fn get_producer(client: &dyn MalSource, id: i64, page: u64) -> Result<Value, MalError> {
    let path = ProducerRequest::new(id, page).path();
    let doc = client.get_html(&path).await?;
    ProducerParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}

/// `MalClient::getProducers()` — full producer list.
pub async fn get_producers(client: &dyn MalSource) -> Result<Value, MalError> {
    let path = ProducersRequest::new().path();
    let doc = client.get_html(&path).await?;
    ProducerListParser::new(doc)
        .model()
        .map_err(|err| MalError::parse_failed(&path, err.to_string()))
}
