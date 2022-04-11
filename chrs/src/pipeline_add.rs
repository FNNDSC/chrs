use anyhow::{bail, Context, Error, Ok, Result};
use chris::pipeline::{CanonPipeline, PossiblyExpandedTreePipeline, TitleIndexedPipeline};
use chris::ChrisClient;
use fs_err as fs;
use fs_err::File;
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
    let file = fs::File::open(filename)?;
    let reader = BufReader::new(file);
    let pipeline = if file_extension == "json" {
        load_json(reader)
    } else if file_extension == "yaml" || file_extension == "yml" {
        load_yaml(reader)
    } else {
        bail!("Unsupported file type: {:?}", file_extension)
    };
    pipeline.with_context(|| format!("Format of {:?} is invalid", filename))
}

fn load_json(reader: BufReader<File>) -> Result<CanonPipeline> {
    let pipeline: PossiblyExpandedTreePipeline = serde_json::from_reader(reader)?;
    Ok(pipeline.into())
}

fn load_yaml(reader: BufReader<File>) -> Result<CanonPipeline> {
    let pipeline: TitleIndexedPipeline = serde_yaml::from_reader(reader)?;
    Ok(pipeline.try_into()?)
}
