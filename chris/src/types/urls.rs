use aliri_braid::braid;

/// A URL to a specific CUBE item, e.g. `plugins/1/` or `pipelines/2/`
#[braid(serde)]
pub struct ItemUrl;

/// A URL to a paginated list of CUBE items, e.g. `plugins/` or `pipelines/`
#[braid(serde)]
pub struct CollectionUrl;

/// Filebrowser API URL, e.g.
/// `https://cube.chrisproject.org/api/v1/filebrowser/`
#[braid(serde)]
pub struct FileBrowserUrl;

/// Filebrowser search API URL, e.g.
/// `https://cube.chrisproject.org/api/v1/filebrowser/search/`
#[braid(serde)]
pub struct FileBrowserSearchUrl;
