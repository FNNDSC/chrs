/// Indicates what part of a CUBE (swift) file path we are looking at.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) enum DescentContext {
    /// Empty path, parent of all files in *ChRIS*
    Root,
    /// Left-most base path, which is either a username or "SERVICES"
    Base,
    /// Second-from-the-left component, which is either "feed_N", "PACS", or "UPLOADS"
    Feed,
    /// A middle component of a plugin instance output file's fname
    /// after the feed and before the "data" folder.
    PluginInstances,
    /// A path which lacks a human-friendly name, e.g. PACS file, uploaded file.
    Data,
}

impl DescentContext {
    pub fn next(self, subfolder: &str) -> Self {
        match self {
            DescentContext::Base => {
                if subfolder.starts_with("feed_") {
                    DescentContext::Feed
                } else {
                    DescentContext::Data
                }
            }
            DescentContext::Feed => DescentContext::PluginInstances,
            DescentContext::PluginInstances => {
                if subfolder == "data" {
                    DescentContext::Data
                } else {
                    DescentContext::PluginInstances
                }
            }
            DescentContext::Data => DescentContext::Data,
            DescentContext::Root => DescentContext::Base,
        }
    }
}
