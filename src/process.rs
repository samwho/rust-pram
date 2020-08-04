use crate::proc;
use lazy_static::lazy_static;
use lasso::{ThreadedRodeo, Spur};
use std::{hash::Hash, sync::Arc, path::Path, fs::File, collections::BTreeMap, io::Read};
use crate::proc::maps::Map;
use anyhow::Result;
use crate::proc::pagemap::Page;

lazy_static! {
    static ref INTERNER: Arc<ThreadedRodeo<Spur>> = Arc::new(ThreadedRodeo::new());
}

#[derive(Debug, Copy, Clone, Eq)]
pub struct Process {
    pub pid: u64,
    cmdline: Spur,
}

impl PartialEq for Process {
    fn eq(&self, other: &Self) -> bool {
        self.pid == other.pid
    }
}

impl PartialOrd for Process {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.pid.partial_cmp(&other.pid)
    }
}

impl Ord for Process {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.pid.cmp(&other.pid)
    }
}

impl Hash for Process {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.pid);
    }
}

impl Process {
    pub fn new(pid: u64) -> Result<Self> {
        let mut file = File::open(
            Path::new("/proc")
                .join(pid.to_string())
                .join("cmdline"),
        )?;
        let mut cmdline = String::new();
        file.read_to_string(&mut cmdline)?;
        Ok(Self { pid, cmdline: INTERNER.get_or_intern(cmdline) })
    }

    pub fn read_pages(&self) -> Result<BTreeMap<Map, Vec<Page>>> {
        let maps = proc::maps::read(&self)?;
        let pages = proc::pagemap::from(self.pid, &maps)?;
        Ok(pages)
    }
}

pub fn all() -> Result<Vec<Process>> {
    let mut procs = Vec::new();
    for result in std::fs::read_dir("/proc")? {
        let dir = result?;
        if !dir.metadata()?.is_dir() {
            continue;
        }

        let path = dir.path();
        let file_name = match path.components().last() {
            Some(name) => name.as_os_str().to_string_lossy(),
            None => continue,
        };

        let pid = match u64::from_str_radix(&file_name, 10) {
            Ok(pid) => pid,
            Err(_) => continue,
        };

        procs.push(Process::new(pid)?);
    }

    Ok(procs)
}
