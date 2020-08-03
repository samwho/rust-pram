use anyhow::Result;
use lazy_static::lazy_static;
use regex::Regex;
use std::{
    convert::TryInto,
    fs::File,
    io::{BufRead, BufReader},
    ops::Range,
    path::Path,
};

lazy_static! {
    static ref MAP_RE: Regex = Regex::new(r"^(?P<from>[0-9a-f]+)-(?P<to>[0-9a-f]+)\s+(?P<permissions>....)\s+(?P<offset>[0-9a-f]+)\s+(?P<dev>..:..)\s+(?P<inode>[0-9]+)\s*(?:(?P<path>.+))?$").unwrap();
}

#[derive(Debug, PartialEq)]
pub struct Map {
    pub address_range: Range<u64>,
    pub permissions: String,
    pub offset: u64,
    pub device: String,
    pub inode: u64,
    pub path: Option<String>,
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
        PageOffsets {
            start: (self.address_range.start / 4096 * 8),
            end: (self.address_range.end / 4096 * 8) - 8,
        }
    }
}

pub fn read(pid: u64) -> Result<Vec<Map>> {
    let path = Path::new("/proc").join(pid.to_string()).join("maps");
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
            address_range: u64::from_str_radix(captures.name("from").unwrap().as_str(), 16)?
                ..u64::from_str_radix(captures.name("to").unwrap().as_str(), 16)?,
            permissions: captures.name("permissions").unwrap().as_str().to_owned(),
            offset: u64::from_str_radix(captures.name("offset").unwrap().as_str(), 16)?,
            device: captures.name("dev").unwrap().as_str().to_owned(),
            inode: captures
                .name("inode")
                .unwrap()
                .as_str()
                .to_owned()
                .parse()?,
            path: captures.name("path").map(|p| p.as_str().to_owned()),
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
                address_range: 2097152..2248704,
                permissions: "r--p".to_owned(),
                offset: 0,
                device: "00:12".to_owned(),
                inode: 281474977421407,
                path: Some("/init".to_owned()),
            }
        );

        Ok(())
    }

    #[test]
    fn test_page_offsets() {
        let offsets: Vec<u64> = Map {
            address_range: 0..0x2000,
            permissions: "".to_owned(),
            offset: 0,
            device: "00:00".to_owned(),
            inode: 0,
            path: None,
        }
        .page_offsets()
        .collect();

        assert_eq!(offsets, vec![0, 8]);
    }
}
