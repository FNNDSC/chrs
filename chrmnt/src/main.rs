use std::path::PathBuf;
use chris::common_types::Username;
use chris::CUBEAuth;
use clap::Parser;

use fuser::{
    FileAttr, FileType, Filesystem, MountOption, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry,
    Request,
};
use libc::ENOENT;
use std::ffi::OsStr;
use std::time::{Duration, UNIX_EPOCH};
use anyhow::Context;
use clap::builder::Str;
use chris::filebrowser::FileBrowserPath;
use futures::TryStreamExt;
use chris::models::Downloadable;

#[derive(Parser)]
#[clap(about = "chrs mount proof-of-concept")]
struct Cli {
    /// CUBE filebrowser path
    path: FileBrowserPath,
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

    let mountpoint = &args.mountpoint;
    let mut options = vec![
        MountOption::RO,
        MountOption::FSName("hello".to_string()),
        // MountOption::AutoUnmount,
        // MountOption::AllowRoot
    ];

    let contents: Vec<String> = chris.file_browser().readdir(&args.path).await?
        .with_context(|| format!("Path not found in ChRIS: {:?}", &args.path))?
        .iter_files()
        .stream()
        .map_ok(|f| f.fname().as_str().rsplit_once('/').unwrap_or(("", f.fname().as_str())).1.to_string())
        .try_collect().await?;
    let fs = HelloFS{ contents: dbg!(contents) };
    fuser::mount2(fs, mountpoint, &options).unwrap();

    Ok(())
}


const TTL: Duration = Duration::from_secs(1); // 1 second

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

const HELLO_TXT_CONTENT: &str = "Hello from chrs!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 17,
    blocks: 1,
    atime: UNIX_EPOCH, // 1970-01-01 00:00:00
    mtime: UNIX_EPOCH,
    ctime: UNIX_EPOCH,
    crtime: UNIX_EPOCH,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
    blksize: 512,
};

struct HelloFS {
    contents: Vec<String>
}

impl Filesystem for HelloFS {
    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {
        if parent == 1 {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        _size: u32,
        _flags: i32,
        _lock: Option<u64>,
        reply: ReplyData,
    ) {
        if ino == 2 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(
        &mut self,
        _req: &Request,
        ino: u64,
        _fh: u64,
        offset: i64,
        mut reply: ReplyDirectory,
    ) {
        if ino != 1 {
            reply.error(ENOENT);
            return;
        }

        let entries = vec![
            (1, FileType::Directory, "."),
            (1, FileType::Directory, ".."),
            (2, FileType::RegularFile, "hello.txt"),
        ];

        for (i, entry) in self.contents.iter().enumerate().skip(offset as usize) {
            // i + 1 means the index of the next entry
            if reply.add((i + 100) as u64, (i + 101) as i64, FileType::RegularFile, entry) {
                break;
            }
        }
        reply.ok();
        dbg!("sent reply");
    }
}
