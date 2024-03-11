use chris::errors::CubeError;

#[derive(thiserror::Error, Debug)]
pub enum FileTransferError {
    #[error(transparent)]
    Cube(#[from] CubeError),
    #[error(transparent)]
    IO(#[from] std::io::Error),
}
