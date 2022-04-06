use serde::Serialize;

/// Query string parameters for paginated GET endpoints.
#[derive(Serialize)]
pub struct PaginationQuery {
    pub limit: u8,
    pub offset: u32,
}
