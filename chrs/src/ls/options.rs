use serde::Serialize;

#[derive(clap::ValueEnum, Copy, Clone, Default, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum WhatToPrint {
    #[default]
    Files,
    Folders,
    All,
}

impl WhatToPrint {
    pub fn should_print_files(&self) -> bool {
        !matches!(&self, Self::Folders)
    }

    pub fn should_print_folders(&self) -> bool {
        !matches!(&self, Self::Files)
    }
}
