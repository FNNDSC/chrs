use aliri_braid::braid;

/// *ChRIS* user's username.
#[braid(serde)]
pub struct Username;

/// Download URL for a file resource.
///
/// # Examples
///
/// - `https://cube.chrisproject.org/api/v1/files/84360/aparc.a2009s+aseg.mgz`
#[braid(serde)]
pub struct FileResourceUrl;

/// File fname.
#[braid(serde)]
pub struct FileResourceFname;

/// Plugin name
#[braid(serde)]
pub struct PluginName;

/// Plugin version
#[braid(serde)]
pub struct PluginVersion;

/// Plugin URL
#[braid(serde)]
pub struct PluginUrl;

/// Container image name of a plugin
#[braid(serde)]
pub struct DockImage;

/// Public source code repository of a plugin
#[braid(serde)]
pub struct PluginRepo;

/// Compute resource name
#[braid(serde)]
pub struct ComputeResourceName;

/// Date in ISO-8601 format.
#[braid(serde)]
pub struct DateString;

/// A path which can be browsed by the file browser API, e.g. `chris/uploads`
#[braid(serde)]
pub struct FileBrowserPath;

// TODO why do I need to convert from FileBrowserPath to FileResourceFname?
// impl From<FileBrowserPath> for FileResourceFname {
//     fn from(p: FileBrowserPath) -> Self {
//         FileResourceFname::new(p.take())
//     }
// }
