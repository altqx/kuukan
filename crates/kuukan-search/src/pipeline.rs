//! Batching helper around [`SearchIndex`].
//!
//! The indexer feeds JMS payloads one by one; Tantivy commits are expensive, so
//! [`IndexPipeline`] writes operations straight into the per-kind writer but
//! only commits when a batch is full or when [`IndexPipeline::flush`] is
//! called. This keeps `kuukan-search` independent from `kuukan-store`: callers
//! own the payload source.

use std::sync::Mutex;

use serde_json::Value;

use crate::error::Result;
use crate::index::SearchIndex;
use crate::schema::EntityKind;

/// Commit after this many queued operations by default.
pub const DEFAULT_BATCH_SIZE: usize = 250;

struct PipelineState {
    /// Operations applied to the writers since the last commit.
    queued_ops: usize,
    batch_size: usize,
}

/// Owns a [`SearchIndex`] and batches writes into commits.
pub struct IndexPipeline {
    index: SearchIndex,
    state: Mutex<PipelineState>,
}

impl IndexPipeline {
    /// Wrap `index` with the default batch size.
    pub fn new(index: SearchIndex) -> IndexPipeline {
        IndexPipeline::with_batch_size(index, DEFAULT_BATCH_SIZE)
    }

    /// Wrap `index` with an explicit batch size (minimum 1).
    pub fn with_batch_size(index: SearchIndex, batch_size: usize) -> IndexPipeline {
        IndexPipeline {
            index,
            state: Mutex::new(PipelineState {
                queued_ops: 0,
                batch_size: batch_size.max(1),
            }),
        }
    }

    /// The underlying index.
    pub fn index(&self) -> &SearchIndex {
        &self.index
    }

    /// Number of operations applied since the last commit.
    pub fn pending(&self) -> usize {
        self.lock_state().queued_ops
    }

    fn lock_state(&self) -> std::sync::MutexGuard<'_, PipelineState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Upsert a payload and commit when the batch is full.
    pub fn index_payload(&self, kind: EntityKind, payload: &Value) -> Result<()> {
        self.index.upsert(kind, payload)?;
        self.bump()
    }

    /// Delete by `mal_id` and commit when the batch is full.
    pub fn remove(&self, kind: EntityKind, mal_id: u64) -> Result<()> {
        self.index.delete(kind, mal_id)?;
        self.bump()
    }

    /// Clear a kind and commit when the batch is full.
    pub fn clear(&self, kind: EntityKind) -> Result<()> {
        self.index.clear(kind)?;
        self.bump()
    }

    fn bump(&self) -> Result<()> {
        let should_flush = {
            let mut state = self.lock_state();
            state.queued_ops += 1;
            state.queued_ops >= state.batch_size
        };
        if should_flush {
            self.flush()?;
        }
        Ok(())
    }

    /// Replace the whole index of `kind` with `items`.
    ///
    /// The clear only becomes visible at the first commit (the final flush when
    /// the rebuild fits in one batch), so a rebuild never exposes an empty
    /// index to concurrent readers.
    ///
    /// Returns the number of indexed items.
    pub fn rebuild_from_iter<'a>(
        &self,
        kind: EntityKind,
        items: impl IntoIterator<Item = &'a Value>,
    ) -> Result<usize> {
        self.index.clear(kind)?;
        self.bump()?;

        let mut count = 0;
        for item in items {
            self.index.upsert(kind, item)?;
            self.bump()?;
            count += 1;
        }
        self.flush()?;
        Ok(count)
    }

    /// Commit every queued operation.
    pub fn flush(&self) -> Result<()> {
        {
            let mut state = self.lock_state();
            state.queued_ops = 0;
        }
        self.index.commit()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn anime(id: u64, title: &str) -> Value {
        json!({
            "mal_id": id,
            "title": title,
            "approved": true,
            "aired": {"from": "2010-01-01T00:00:00+00:00", "to": null},
        })
    }

    #[test]
    fn batches_and_flushes() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();
        let pipeline = IndexPipeline::with_batch_size(index, 2);

        pipeline
            .index_payload(EntityKind::Anime, &anime(1, "One"))
            .unwrap();
        assert_eq!(pipeline.pending(), 1);
        pipeline
            .index_payload(EntityKind::Anime, &anime(2, "Two"))
            .unwrap();
        // Batch size reached: auto-flushed and searchable.
        assert_eq!(pipeline.pending(), 0);
        assert_eq!(
            pipeline
                .index()
                .searcher(EntityKind::Anime)
                .unwrap()
                .num_docs(),
            2
        );

        pipeline
            .index_payload(EntityKind::Anime, &anime(3, "Three"))
            .unwrap();
        assert_eq!(pipeline.pending(), 1);
        pipeline.flush().unwrap();
        assert_eq!(
            pipeline
                .index()
                .searcher(EntityKind::Anime)
                .unwrap()
                .num_docs(),
            3
        );
    }

    #[test]
    fn rebuild_replaces_everything() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();
        let pipeline = IndexPipeline::new(index);

        pipeline
            .rebuild_from_iter(EntityKind::Anime, [anime(1, "One")].iter())
            .unwrap();
        pipeline
            .rebuild_from_iter(
                EntityKind::Anime,
                [anime(7, "Seven"), anime(8, "Eight")].iter(),
            )
            .unwrap();

        assert_eq!(
            pipeline
                .index()
                .searcher(EntityKind::Anime)
                .unwrap()
                .num_docs(),
            2
        );
    }

    #[test]
    fn rebuild_is_bounded_and_visible_after_flush() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();
        // Batch size 2 forces intermediate commits during the rebuild.
        let pipeline = IndexPipeline::with_batch_size(index, 2);

        let items: Vec<Value> = (0..5).map(|id| anime(id, "X")).collect();
        pipeline
            .rebuild_from_iter(EntityKind::Anime, items.iter())
            .unwrap();
        assert_eq!(pipeline.pending(), 0);
        assert_eq!(
            pipeline
                .index()
                .searcher(EntityKind::Anime)
                .unwrap()
                .num_docs(),
            5
        );
    }
}
