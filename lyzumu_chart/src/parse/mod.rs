use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

use anyhow::Result;

pub mod lzm;

pub(crate) fn read_lines<P>(filename: P) -> Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
