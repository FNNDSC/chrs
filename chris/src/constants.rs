use crate::api::{PluginName, PluginVersion};
use lazy_static::lazy_static;

lazy_static! {
    pub(crate) static ref DIRCOPY_NAME: PluginName = PluginName::from("pl-dircopy");
    pub(crate) static ref DIRCOPY_VERSION: PluginVersion = PluginVersion::from("2.1.1");
}
