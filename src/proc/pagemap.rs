use crate::proc::maps::Map;
use anyhow::{Context, Result};
use std::{
    fmt::Debug,
    fs::File,
    io::{BufReader, Read, Seek, SeekFrom},
    path::Path, collections::BTreeMap,
};

bitfield! {
    #[derive(PartialEq)]
    pub struct Page(u64);
    pub in_ram, _: 63;
    pub in_swap, _: 62;
    pub is_file_mapped, _: 61;
    pub is_shared_anonymous, _: 61;
    pub is_exclusively_mapped, _: 56;
    pub is_soft_dirty, _: 55;
    pub u64, page_frame_number, _: 54, 0;
}

impl Debug for Page {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Page {{ in_ram: {}, in_swap: {}, is_file_mapped: {}, is_shared_anonymous: {}, is_exclusively_mapped: {}, is_soft_dirty: {}, page_frame_number: {} }}", self.in_ram(), self.in_swap(), self.is_file_mapped(), self.is_shared_anonymous(), self.is_exclusively_mapped(), self.is_soft_dirty(), self.page_frame_number())
    }
}

pub fn from(pid: u64, maps: &[Map]) -> Result<BTreeMap<Map, Vec<Page>>>
{
    let path = Path::new("/proc").join(pid.to_string()).join("pagemap");
    let file = File::open(path)?;
    let mut read = BufReader::new(file);
    let mut buf = [0 as u8; 8];
    let mut ret = BTreeMap::new();

    println!("maps len: {}", maps.len());

    for map in maps {
        let mut pages = Vec::new();
        for offset in map.page_offsets() {
            read.seek(SeekFrom::Start(offset))
                .context(format!("failed to seek to page {} in pagemap", offset))?;
            read.read_exact(&mut buf)
                .context(format!("failed to read from page {} in pagemap", offset))?;
            pages.push(Page(u64::from_le_bytes(buf)));
        }
        ret.insert(map.clone(), pages);
    }

    println!("ret len: {}", ret.len());

    Ok(ret)
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_zero() {
        let page = Page(0);
        assert_eq!(page.in_ram(), false);
        assert_eq!(page.in_swap(), false);
        assert_eq!(page.is_file_mapped(), false);
        assert_eq!(page.is_shared_anonymous(), false);
        assert_eq!(page.is_exclusively_mapped(), false);
        assert_eq!(page.is_soft_dirty(), false);
        assert_eq!(page.page_frame_number(), 0);
    }

    #[test]
    fn test_one() {
        let page = Page(1);
        assert_eq!(page.in_ram(), false);
        assert_eq!(page.in_swap(), false);
        assert_eq!(page.is_file_mapped(), false);
        assert_eq!(page.is_shared_anonymous(), false);
        assert_eq!(page.is_exclusively_mapped(), false);
        assert_eq!(page.is_soft_dirty(), false);
        assert_eq!(page.page_frame_number(), 1);
    }
}
