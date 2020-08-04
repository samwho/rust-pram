mod proc;
mod process;

use maplit::btreeset;
use rayon::prelude::*;
use process::Process;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate bitfield;

use anyhow::Result;
use std::{
    collections::{BTreeMap, BTreeSet},
};

#[derive(Debug, Ord, PartialOrd, Eq, PartialEq)]
struct PageRange {
    from: u64,
    to: u64,
}

fn compress(pages: BTreeMap<u64, BTreeSet<Process>>) -> BTreeMap<PageRange, BTreeSet<Process>> {
    let mut ret = BTreeMap::new();

    let mut start = 0;
    let mut prev = 0;

    for (addr, pids) in pages {
        if start == 0 {
            start = addr;
            prev = addr;
            continue;
        }

        if prev == addr - 1 {
            prev += 1;
            continue;
        }

        ret.insert(
            PageRange {
                from: start,
                to: addr,
            },
            pids,
        );
        start = 0;
    }

    ret
}

fn all_pages(procs: &Vec<Process>) -> BTreeMap<u64, BTreeSet<Process>> {
    procs
        .par_iter()
        .map(|proc| {
            let mut page_map: BTreeMap<u64, BTreeSet<Process>> = BTreeMap::new();
            let maps = proc
                .read_pages()
                .expect(&format!("failed to read pages for pid {}", proc.pid));
            for (map, pages) in maps {
                for page in pages {
                    if !page.in_ram() {
                        continue;
                    }

                    page_map
                        .entry(page.page_frame_number())
                        .or_insert_with(|| btreeset! {})
                        .insert(proc.to_owned());
                }
            }
            page_map
        })
        .reduce(
            || BTreeMap::new(),
            |mut a, b| {
                for (k, v) in b {
                    let set = a.entry(k).or_insert_with(|| BTreeSet::new());
                    set.extend(v);
                }
                a
            },
        )
}

fn main() -> Result<()> {
    let processes = process::all()?;
    let all_pages = all_pages(&processes);

    for (page_range, pids) in compress(all_pages) {
        println!(
            "0x{:x}-0x{:x} -- {:?}",
            page_range.from * 4096,
            page_range.to * 4096,
            pids
        );
    }

    Ok(())
}
