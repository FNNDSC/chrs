use anyhow::{bail, Context, Error, Ok, Result};
use chris::pipeline::{
    CanonPipeline, ExpandedTreePipeline, PossiblyExpandedTreePipeline, TitleIndexedPipeline,
};
use chris::ChrisClient;
use fs_err as fs;
use fs_err::File;
use std::io::{BufReader, BufWriter};
use std::path::Path;

pub async fn add_pipeline(client: &ChrisClient, file: &Path) -> Result<()> {
    let pipeline: CanonPipeline = read_pipeline_file(file)?.into();
    let uploaded = client.upload_pipeline(&pipeline).await?;
    println!("{}", uploaded.url);
    Ok(())
}

pub async fn convert_pipeline(expand: bool, src: &Path, dst: &Path) -> Result<()> {
    let pipeline = read_pipeline_file(src)?;
    let file_extension = dst
        .extension()
        .ok_or_else(|| Error::msg(format!("Unknown file type of: {:?}", src)))?;
    let file = fs::File::create(dst)?;
    let writer = BufWriter::new(file);

    if file_extension == "json" {
        if expand {
            serde_json::to_writer_pretty(writer, &pipeline)?;
        } else {
            let canonicalized: CanonPipeline = pipeline.into();
            serde_json::to_writer(writer, &canonicalized)?;
        }
    } else if file_extension == "yaml" || file_extension == "yml" {
        let converted: TitleIndexedPipeline = pipeline.try_into()?;
        serde_yaml::to_writer(writer, &converted)?;
    } else {
        bail!("Unsupported output format: {:?}", file_extension);
    }
    Ok(())
}

fn read_pipeline_file(filename: &Path) -> Result<ExpandedTreePipeline> {
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

fn load_json(reader: BufReader<File>) -> Result<ExpandedTreePipeline> {
    let pipeline: PossiblyExpandedTreePipeline = serde_json::from_reader(reader)?;
    Ok(pipeline.try_into()?)
}

fn load_yaml(reader: BufReader<File>) -> Result<ExpandedTreePipeline> {
    let pipeline: TitleIndexedPipeline = serde_yaml::from_reader(reader)?;
    Ok(pipeline.try_into()?)
}
