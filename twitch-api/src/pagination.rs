use serde::Deserialize;

use crate::secret::Secret;

#[derive(Debug, Deserialize)]
pub struct Pagination {
    /// The cursor used to get the next page of results. Use the cursor to set the request’s after query parameter.
    #[serde(default)]
    pub cursor: Option<Secret>,
}
