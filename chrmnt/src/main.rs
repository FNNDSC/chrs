use std::ffi::CStr;
use chris::{ChrisClient, CUBEAuth};
use clap::Parser;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use fuse_backend_rs::abi::fuse_abi::{CreateIn, OpenOptions, SetattrValid, stat64};
use fuse_backend_rs::api::BackendFileSystem;
use chris::common_types::{CUBEApiUrl, Username};
use fuse_backend_rs::api::filesystem::{AsyncFileSystem, AsyncZeroCopyReader, AsyncZeroCopyWriter, Context, Entry, FileSystem};

#[derive(Parser)]
#[clap(about = "chrs mount proof-of-concept")]
struct Cli {
    /// CUBE filebrowser path
    path: String,
    /// mount point
    mountpoint: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Cli = Cli::parse();
    let chris = CUBEAuth::new(
         "https://cube.chrisproject.org/api/v1/".to_string().parse().unwrap(),
        Username::new("chris".to_string()),
        "chris1234".to_string()
    ).into_client().await?;


    let mut daemon = Daemon::new("/hdd/jennings/hello-root", "/hdd/jennings/mnt", 2).unwrap();
    daemon.mount().unwrap();
    std::thread::sleep(std::time::Duration::from_secs(10));
    daemon.umount().unwrap();
    Ok(())



    // let mut fuse_session = FuseSession::new(
    //     Path::new("/hdd/jennings/mnt"),
    //     "fsname",
    //     "",
    //     true
    // )?;
    //
    // fuse_session.mount().unwrap();

}



use fuse_backend_rs::api::{server::Server, Vfs, VfsOptions};
use fuse_backend_rs::passthrough::{Config, PassthroughFs};
use fuse_backend_rs::transport::{FuseChannel, FuseSession};

/// A fusedev daemon example
#[allow(dead_code)]
pub struct Daemon {
    mountpoint: String,
    server: Arc<Server<Arc<Vfs>>>,
    thread_cnt: u32,
    session: Option<FuseSession>,
}

#[allow(dead_code)]
impl Daemon {
    /// Creates a fusedev daemon instance
    pub fn new(src: &str, mountpoint: &str, thread_cnt: u32) -> std::io::Result<Self> {
        // create vfs
        let vfs = Vfs::new(VfsOptions {
            no_open: false,
            no_opendir: false,
            ..Default::default()
        });

        // create passthrough fs
        let mut cfg = Config::default();
        cfg.root_dir = src.to_string();
        cfg.do_import = false;
        let fs = PassthroughFs::<()>::new(cfg).unwrap();
        fs.import().unwrap();

        // attach passthrough fs to vfs root
        vfs.mount(Box::new(fs), "/").unwrap();

        Ok(Daemon {
            mountpoint: mountpoint.to_string(),
            server: Arc::new(Server::new(Arc::new(vfs))),
            thread_cnt,
            session: None,
        })
    }

    /// Mounts a fusedev daemon to the mountpoint, then start service threads to handle
    /// FUSE requests.
    pub fn mount(&mut self) -> std::io::Result<()> {
        let mut se =
            FuseSession::new(Path::new(&self.mountpoint), "passthru_example", "", false).unwrap();
        se.mount().unwrap();
        for _ in 0..self.thread_cnt {
            let mut server = FuseServer {
                server: self.server.clone(),
                ch: se.new_channel().unwrap(),
            };
            let _thread = thread::Builder::new()
                .name("fuse_server".to_string())
                .spawn(move || {
                    eprintln!("new fuse thread");
                    let _ = server.svc_loop();
                    eprintln!("fuse service thread exits");
                })
                .unwrap();
        }
        self.session = Some(se);
        Ok(())
    }

    /// Umounts and destroies a fusedev daemon
    pub fn umount(&mut self) -> std::io::Result<()> {
        if let Some(mut se) = self.session.take() {
            se.umount().unwrap();
            se.wake().unwrap();
        }
        Ok(())
    }
}

impl Drop for Daemon {
    fn drop(&mut self) {
        let _ = self.umount();
    }
}

struct FuseServer {
    server: Arc<Server<Arc<Vfs>>>,
    ch: FuseChannel,
}

impl FuseServer {
    fn svc_loop(&mut self) -> std::io::Result<()> {
        // Given error EBADF, it means kernel has shut down this session.
        let _ebadf = std::io::Error::from_raw_os_error(libc::EBADF);
        loop {
            if let Some((reader, writer)) = self
                .ch
                .get_request()
                .map_err(|_| std::io::Error::from_raw_os_error(libc::EINVAL))?
            {
                if let Err(e) = self
                    .server
                    .handle_message(reader, writer.into(), None, None)
                {
                    match e {
                        fuse_backend_rs::Error::EncodeMessage(_ebadf) => {
                            break;
                        }
                        _ => {
                            eprintln!("Handling fuse message failed");
                            continue;
                        }
                    }
                }
            } else {
                eprintln!("fuse server exits");
                break;
            }
        }
        Ok(())
    }
}

struct CubeFilesystem {
    chris: ChrisClient
}

impl FileSystem for CubeFilesystem {
    type Inode = u64;
    type Handle = u64;
}

#[async_trait::async_trait]
impl AsyncFileSystem for CubeFilesystem {
    async fn async_lookup(&self, ctx: &Context, parent: Self::Inode, name: &CStr) -> std::io::Result<Entry> {
        todo!()
    }

    async fn async_getattr(&self, ctx: &Context, inode: Self::Inode, handle: Option<Self::Handle>) -> std::io::Result<(stat64, Duration)> {
        todo!()
    }

    async fn async_setattr(&self, ctx: &Context, inode: Self::Inode, attr: stat64, handle: Option<Self::Handle>, valid: SetattrValid) -> std::io::Result<(stat64, Duration)> {
        todo!()
    }

    async fn async_open(&self, ctx: &Context, inode: Self::Inode, flags: u32, fuse_flags: u32) -> std::io::Result<(Option<Self::Handle>, OpenOptions)> {
        todo!()
    }

    async fn async_create(&self, ctx: &Context, parent: Self::Inode, name: &CStr, args: CreateIn) -> std::io::Result<(Entry, Option<Self::Handle>, OpenOptions)> {
        todo!()
    }

    async fn async_read(&self, ctx: &Context, inode: Self::Inode, handle: Self::Handle, w: &mut (dyn AsyncZeroCopyWriter + Send), size: u32, offset: u64, lock_owner: Option<u64>, flags: u32) -> std::io::Result<usize> {
        todo!()
    }

    async fn async_write(&self, ctx: &Context, inode: Self::Inode, handle: Self::Handle, r: &mut (dyn AsyncZeroCopyReader + Send), size: u32, offset: u64, lock_owner: Option<u64>, delayed_write: bool, flags: u32, fuse_flags: u32) -> std::io::Result<usize> {
        todo!()
    }

    async fn async_fsync(&self, ctx: &Context, inode: Self::Inode, datasync: bool, handle: Self::Handle) -> std::io::Result<()> {
        todo!()
    }

    async fn async_fallocate(&self, ctx: &Context, inode: Self::Inode, handle: Self::Handle, mode: u32, offset: u64, length: u64) -> std::io::Result<()> {
        todo!()
    }

    async fn async_fsyncdir(&self, ctx: &Context, inode: Self::Inode, datasync: bool, handle: Self::Handle) -> std::io::Result<()> {
        todo!()
    }
}