mod proc;

#[macro_use]
extern crate anyhow;

#[macro_use]
extern crate bitfield;

use anyhow::Result;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
    pid: u64,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let maps = proc::maps::read(opt.pid)?;
    let pages = proc::pagemap::from(opt.pid, &maps)?;

    for page in pages {
        println!("{:?}", page);
    }

    Ok(())
}
