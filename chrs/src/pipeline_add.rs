use anyhow::{bail, Context, Error, Ok, Result};
use chris::pipeline::{CanonPipeline, PossiblyExpandedTreePipeline};
use chris::ChrisClient;
use fs_err as fs;
use std::io::BufReader;
use std::path::Path;

pub async fn add_pipeline(client: &ChrisClient, file: &Path) -> Result<()> {
    let pipeline = read_pipeline_file(file)?;
    let uploaded = client.upload_pipeline(&pipeline).await?;
    println!("{}", uploaded.url);
    Ok(())
}

pub fn read_pipeline_file(filename: &Path) -> Result<CanonPipeline> {
    let file_extension = filename
        .extension()
        .ok_or_else(|| Error::msg(format!("Unknown file type of: {:?}", filename)))?;
    if file_extension != "json" {
        bail!("Unsupported file type: {:?}", file_extension);
    }
    let file = fs::File::open(filename)?;
    let reader = BufReader::new(file);
    let pipeline: PossiblyExpandedTreePipeline = serde_json::from_reader(reader)
        .with_context(|| format!("Format of {:?} is invalid", filename))?;
    Ok(pipeline.into())
}
