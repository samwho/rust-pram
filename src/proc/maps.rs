use anyhow::Result;
use lazy_static::lazy_static;
use nix::unistd::{sysconf, SysconfVar::PAGE_SIZE};
use regex::Regex;
use std::{
    convert::TryInto,
    fs::File,
    io::{BufRead, BufReader},
    mem::size_of,
    path::Path, sync::Arc, hash::Hash, 
};
use lasso::{ThreadedRodeo, MiniSpur};
use crate::process::Process;

lazy_static! {
    static ref MAP_RE: Regex = Regex::new(r"^(?P<from>[0-9a-f]+)-(?P<to>[0-9a-f]+)\s+(?P<permissions>....)\s+(?P<offset>[0-9a-f]+)\s+(?P<dev>..:..)\s+(?P<inode>[0-9]+)\s*(?:(?P<path>.+))?$").unwrap();
    static ref INTERNER: Arc<ThreadedRodeo<MiniSpur>> = Arc::new(ThreadedRodeo::new());
}

#[derive(Debug, PartialEq, Ord, PartialOrd, Eq, Copy, Clone, Hash)]
pub struct Range {
    start: u64,
    end: u64,
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub struct Map {
    pub address_range: Range,
    permissions: MiniSpur,
    pub offset: u64,
    device: MiniSpur,
    pub inode: u64,
    path: Option<MiniSpur>,
}

impl Hash for Map {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_u64(self.address_range.start);
        state.write_u64(self.address_range.end);
    }
}

impl PartialOrd for Map {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.address_range
            .start
            .partial_cmp(&other.address_range.start)
    }
}

impl Ord for Map {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.address_range
            .start
            .cmp(&other.address_range.start)
    }
}

pub struct PageOffsets {
    start: u64,
    end: u64,
}

impl Iterator for PageOffsets {
    type Item = u64;
    fn next(&mut self) -> Option<Self::Item> {
        let current = self.start;
        if current > self.end {
            None
        } else {
            self.start += 8;
            Some(current)
        }
    }
}

impl Map {
    pub fn page_offsets(&self) -> PageOffsets {
        let u64_size = size_of::<u64>() as u64;
        let page_size = sysconf(PAGE_SIZE).unwrap().unwrap() as u64;
        PageOffsets {
            start: (self.address_range.start / page_size * u64_size),
            end: (self.address_range.end / page_size * u64_size) - u64_size,
        }
    }

    pub fn path(&self) -> Option<&str> {
        self.path.map(|path| INTERNER.resolve(&path))
    }
}

pub fn read(process: &Process) -> Result<Vec<Map>> {
    let path = Path::new("/proc").join(process.pid.to_string()).join("maps");
    let file = File::open(path)?;
    let read = BufReader::new(file);

    let mut maps = Vec::new();

    for res in read.lines() {
        maps.push(res?.as_str().try_into()?);
    }

    Ok(maps)
}

impl TryInto<Map> for &str {
    type Error = anyhow::Error;
    fn try_into(self) -> std::result::Result<Map, Self::Error> {
        let captures = match MAP_RE.captures(self) {
            Some(c) => c,
            None => {
                return Err(anyhow!("failed to match against line: \"{}\"", self));
            }
        };

        Ok(Map {
            address_range: Range { start: u64::from_str_radix(captures.name("from").unwrap().as_str(), 16)?, end: u64::from_str_radix(captures.name("to").unwrap().as_str(), 16)? },
            permissions: INTERNER.get_or_intern(captures.name("permissions").unwrap().as_str()),
            offset: u64::from_str_radix(captures.name("offset").unwrap().as_str(), 16)?,
            device: INTERNER.get_or_intern(captures.name("dev").unwrap().as_str()),
            inode: captures
                .name("inode")
                .unwrap()
                .as_str()
                .to_owned()
                .parse()?,
            path: captures.name("path").map(|p| INTERNER.get_or_intern(p.as_str())),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple() -> Result<()> {
        let line = "00200000-00225000 r--p 00000000 00:12 281474977421407                    /init";
        let map: Map = line.try_into()?;

        assert_eq!(
            map,
            Map {
                address_range: Range { start: 2097152, end: 2248704 },
                permissions: INTERNER.get_or_intern("r--p"),
                offset: 0,
                device: INTERNER.get_or_intern("00:12"),
                inode: 281474977421407,
                path: Some(INTERNER.get_or_intern("/init")),
            }
        );

        Ok(())
    }

    #[test]
    fn test_page_offsets() {
        let offsets: Vec<u64> = Map {
            address_range: Range { start: 0, end: 0x2000 },
            permissions: INTERNER.get_or_intern(""),
            offset: 0,
            device: INTERNER.get_or_intern("00:00"),
            inode: 0,
            path: None,
        }
        .page_offsets()
        .collect();

        assert_eq!(offsets, vec![0, 8]);
    }
}
