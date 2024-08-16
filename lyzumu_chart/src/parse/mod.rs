use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use anyhow::Result;

// pub mod ksh;
pub mod lzm;
// pub mod osu;

pub(crate) fn read_lines<P>(filename: P) -> Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
