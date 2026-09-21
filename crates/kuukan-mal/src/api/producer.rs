//! Frozen producer entry points.

use serde_json::Value;

use crate::error::MalError;
use crate::parser::producer::{ProducerListParser, ProducerParser};
use crate::request::producer::{ProducerRequest, ProducersRequest};
use crate::source::MalSource;

/// `MalClient::getProducer()` — `/anime/producer/{id}`.
pub async fn get_producer(client: &dyn MalSource, id: i64, page: u64) -> Result<Value, MalError> {
    super::fetch_then(client, ProducerRequest::new(id, page), |doc| {
        ProducerParser::new(doc).model()
    })
    .await
}

/// `MalClient::getProducers()` — full producer list.
pub async fn get_producers(client: &dyn MalSource) -> Result<Value, MalError> {
    super::fetch_then(client, ProducersRequest::new(), |doc| {
        ProducerListParser::new(doc).model()
    })
    .await
}
