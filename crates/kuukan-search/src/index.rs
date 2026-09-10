//! Directory-backed set of Tantivy indexes.
//!
//! [`SearchIndex`] owns one sub-index per [`EntityKind`], stored in
//! `<root>/<kind>/` (e.g. `<root>/anime`). Writers are pooled per kind behind a
//! `Mutex`; readers use [`ReloadPolicy::OnCommitWithDelay`] so committed
//! documents become visible shortly after [`SearchIndex::commit`].

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, MutexGuard};

use serde_json::Value;
use tantivy::schema::TantivyDocument;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Searcher, Term};

use crate::error::Result;
use crate::schema::{build_document, read_mal_id, EntityKind, EntitySchema};

/// Memory budget for each indexer (one thread per kind).
const WRITER_MEMORY_BUDGET: usize = 50_000_000;

/// Number of indexer threads per kind.
const WRITER_THREADS: usize = 1;

/// A set of Tantivy indexes, one per entity kind.
///
/// Deletes and upserts are buffered in the per-kind writer and only become
/// searchable after [`SearchIndex::commit`] (or a pipeline flush).
pub struct SearchIndex {
    root: PathBuf,
    kinds: HashMap<EntityKind, KindIndex>,
}

struct KindIndex {
    schema: EntitySchema,
    index: Index,
    writer: Mutex<IndexWriter>,
    reader: IndexReader,
    dirty: AtomicBool,
}

impl KindIndex {
    fn open(root: &Path, kind: EntityKind) -> Result<KindIndex> {
        let dir = root.join(kind.as_str());
        fs::create_dir_all(&dir)?;

        let schema = EntitySchema::build(kind);
        let index = open_or_create(&dir, &schema)?;

        let writer = index
            .writer_with_num_threads::<TantivyDocument>(WRITER_THREADS, WRITER_MEMORY_BUDGET)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(KindIndex {
            schema,
            index,
            writer: Mutex::new(writer),
            reader,
            dirty: AtomicBool::new(false),
        })
    }

    fn lock_writer(&self) -> MutexGuard<'_, IndexWriter> {
        // A poisoned mutex only means a previous writer operation panicked;
        // Tantivy writers stay usable, so recover instead of propagating.
        self.writer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn commit(&self) -> Result<()> {
        if !self.dirty.swap(false, Ordering::SeqCst) {
            return Ok(());
        }
        let result = {
            let mut writer = self.lock_writer();
            writer.commit().map(|_| ())
        };
        match result {
            Ok(()) => {
                // `OnCommitWithDelay` is asynchronous; reload synchronously so
                // the committed documents are searchable right away.
                self.reader.reload()?;
                Ok(())
            }
            Err(error) => {
                self.dirty.store(true, Ordering::SeqCst);
                Err(error.into())
            }
        }
    }
}

/// Open (or create) an index directory with the expected schema.
///
/// When an existing index has a different schema (e.g. the crate was upgraded)
/// it is recreated: the search index is a rebuilt-able cache and the indexer
/// repopulates it.
fn open_or_create(dir: &Path, schema: &EntitySchema) -> Result<Index> {
    let meta = dir.join("meta.json");
    if !meta.exists() {
        return Ok(Index::create_in_dir(dir, schema.schema().clone())?);
    }

    let index = Index::open_in_dir(dir)?;
    if index.schema() == *schema.schema() {
        return Ok(index);
    }

    tracing::warn!(
        kind = schema.kind().as_str(),
        path = %dir.display(),
        "search index schema changed, recreating it"
    );
    drop(index);
    fs::remove_dir_all(dir)?;
    fs::create_dir_all(dir)?;
    Ok(Index::create_in_dir(dir, schema.schema().clone())?)
}

impl SearchIndex {
    /// Open or create all sub-indexes under `root`.
    pub fn open(root: impl AsRef<Path>) -> Result<SearchIndex> {
        let root = root.as_ref().to_path_buf();
        fs::create_dir_all(&root)?;

        let mut kinds = HashMap::with_capacity(EntityKind::ALL.len());
        for kind in EntityKind::ALL {
            kinds.insert(kind, KindIndex::open(&root, kind)?);
        }

        Ok(SearchIndex { root, kinds })
    }

    /// Root directory of the index set.
    pub fn root(&self) -> &Path {
        &self.root
    }

    fn kind_index(&self, kind: EntityKind) -> Result<&KindIndex> {
        self.kinds
            .get(&kind)
            .ok_or_else(|| crate::error::SearchError::UnknownKind(kind.as_str().to_owned()))
    }

    /// Schema of a kind.
    pub fn schema(&self, kind: EntityKind) -> Result<&EntitySchema> {
        Ok(&self.kind_index(kind)?.schema)
    }

    /// Tantivy index of a kind.
    pub fn index(&self, kind: EntityKind) -> Result<&Index> {
        Ok(&self.kind_index(kind)?.index)
    }

    /// Current searcher of a kind.
    pub fn searcher(&self, kind: EntityKind) -> Result<Searcher> {
        Ok(self.kind_index(kind)?.reader.searcher())
    }

    /// Add or replace a document by `mal_id`.
    ///
    /// Not visible to searchers until [`SearchIndex::commit`].
    pub fn upsert(&self, kind: EntityKind, payload: &Value) -> Result<()> {
        let kind_index = self.kind_index(kind)?;
        let mal_id = read_mal_id(payload).ok_or_else(|| {
            crate::error::SearchError::InvalidPayload("missing mal_id".to_owned())
        })?;
        let document = build_document(&kind_index.schema, payload)?;

        let writer = kind_index.lock_writer();
        writer.delete_term(Term::from_field_u64(
            kind_index.schema.mal_id_field(),
            mal_id,
        ));
        writer.add_document(document)?;
        kind_index.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Add or replace many documents without committing.
    ///
    /// Returns the number of upserted documents.
    pub fn upsert_many<'a>(
        &self,
        kind: EntityKind,
        items: impl IntoIterator<Item = &'a Value>,
    ) -> Result<usize> {
        let mut count = 0;
        for item in items {
            self.upsert(kind, item)?;
            count += 1;
        }
        Ok(count)
    }

    /// Delete a document by `mal_id`. Not visible until commit.
    pub fn delete(&self, kind: EntityKind, mal_id: u64) -> Result<()> {
        let kind_index = self.kind_index(kind)?;
        let writer = kind_index.lock_writer();
        writer.delete_term(Term::from_field_u64(
            kind_index.schema.mal_id_field(),
            mal_id,
        ));
        kind_index.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Remove every document of a kind. Not visible until commit.
    pub fn clear(&self, kind: EntityKind) -> Result<()> {
        let kind_index = self.kind_index(kind)?;
        let writer = kind_index.lock_writer();
        writer.delete_all_documents()?;
        kind_index.dirty.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Commit pending operations of every kind.
    pub fn commit(&self) -> Result<()> {
        for kind in EntityKind::ALL {
            self.kind_index(kind)?.commit()?;
        }
        Ok(())
    }

    /// Commit pending operations of one kind.
    pub fn commit_kind(&self, kind: EntityKind) -> Result<()> {
        self.kind_index(kind)?.commit()
    }

    /// Whether a kind has uncommitted operations.
    pub fn is_dirty(&self, kind: EntityKind) -> bool {
        self.kind_index(kind)
            .map(|k| k.dirty.load(Ordering::SeqCst))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tantivy::schema::Value as TantivyValue;

    fn anime(id: u64, title: &str) -> Value {
        json!({
            "mal_id": id,
            "title": title,
            "approved": true,
            "aired": {"from": "2010-01-01T00:00:00+00:00", "to": null},
        })
    }

    #[test]
    fn upsert_commit_and_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();

        index
            .upsert_many(
                EntityKind::Anime,
                [anime(1, "One Piece"), anime(2, "Naruto")].iter(),
            )
            .unwrap();
        assert!(index.is_dirty(EntityKind::Anime));
        index.commit().unwrap();
        assert!(!index.is_dirty(EntityKind::Anime));

        let searcher = index.searcher(EntityKind::Anime).unwrap();
        assert_eq!(searcher.num_docs(), 2);
        drop(index);

        let reopened = SearchIndex::open(dir.path()).unwrap();
        assert_eq!(reopened.searcher(EntityKind::Anime).unwrap().num_docs(), 2);
    }

    #[test]
    fn upsert_replaces_by_mal_id() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();

        index.upsert(EntityKind::Anime, &anime(1, "Old")).unwrap();
        index.commit().unwrap();
        index.upsert(EntityKind::Anime, &anime(1, "New")).unwrap();
        index.commit().unwrap();

        let searcher = index.searcher(EntityKind::Anime).unwrap();
        assert_eq!(searcher.num_docs(), 1);

        let schema = index.schema(EntityKind::Anime).unwrap();
        let query = tantivy::query::AllQuery;
        let top = searcher
            .search(
                &query,
                &tantivy::collector::TopDocs::with_limit(1).order_by_score(),
            )
            .unwrap();
        let doc: TantivyDocument = searcher.doc(top[0].1).unwrap();
        let payload = doc
            .get_first(schema.payload_field())
            .unwrap()
            .as_str()
            .unwrap();
        assert!(payload.contains("\"New\""));
    }

    #[test]
    fn delete_and_clear() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();
        index
            .upsert_many(EntityKind::Anime, [anime(1, "One"), anime(2, "Two")].iter())
            .unwrap();
        index.commit().unwrap();

        index.delete(EntityKind::Anime, 1).unwrap();
        index.commit().unwrap();
        assert_eq!(index.searcher(EntityKind::Anime).unwrap().num_docs(), 1);

        index.clear(EntityKind::Anime).unwrap();
        index.commit().unwrap();
        assert_eq!(index.searcher(EntityKind::Anime).unwrap().num_docs(), 0);
    }

    #[test]
    fn missing_mal_id_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let index = SearchIndex::open(dir.path()).unwrap();
        let error = index
            .upsert(EntityKind::Anime, &json!({"title": "No id"}))
            .unwrap_err();
        assert!(error.to_string().contains("mal_id"));
    }
}
