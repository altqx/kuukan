//! Pagination envelope, ported from `app/Http/Resources/V4/ResultsResource.php`
//! and the `pagination_plus` schema in the OpenAPI contract.

use serde::{Deserialize, Serialize};

/// `pagination.items` for search-style endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageItems {
    pub count: u64,
    pub total: u64,
    pub per_page: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pagination {
    pub last_visible_page: u64,
    pub has_next_page: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_page: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<PageItems>,
}

impl Pagination {
    /// List endpoints (e.g. `/anime/{id}/characters`) only carry these two fields.
    pub fn list(last_visible_page: u64, has_next_page: bool) -> Self {
        Pagination {
            last_visible_page,
            has_next_page,
            current_page: None,
            items: None,
        }
    }

    /// Search endpoints carry the full pagination block.
    pub fn search(
        last_visible_page: u64,
        has_next_page: bool,
        current_page: u64,
        count: u64,
        total: u64,
        per_page: u64,
    ) -> Self {
        Pagination {
            last_visible_page,
            has_next_page,
            current_page: Some(current_page),
            items: Some(PageItems {
                count,
                total,
                per_page,
            }),
        }
    }
}

/// Compute pagination for a result set, mirroring `DefaultBuilderPaginatorService`.
pub fn paginate(total: u64, page: u64, per_page: u64) -> Pagination {
    let per_page = per_page.max(1);
    let last_page = ((total as f64) / (per_page as f64)).ceil().max(1.0) as u64;
    let count = total.saturating_sub((page.saturating_sub(1)) * per_page).min(per_page);
    Pagination::search(
        last_page,
        page < last_page,
        page,
        count,
        total,
        per_page,
    )
}
