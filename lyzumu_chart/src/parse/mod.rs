use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use anyhow::Result;

pub mod dtx;
pub mod ksh;
pub mod osu;
pub mod pms;
pub mod sm;
pub mod sus;

pub(crate) fn read_lines<P>(filename: P) -> Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
