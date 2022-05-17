use std::fs::File;
use std::io;
use std::path::Path;
use core::transaction::Transaction;

pub fn read_from_file<T>(file_path: T) -> Result<impl Iterator<Item=Result<Transaction, csv::Error>>, io::Error>
    where T: AsRef<Path>
{
    read_from_csv(File::open(file_path)?)
}

pub fn read_from_csv<R>(reader: R) -> Result<impl Iterator<Item=Result<Transaction, csv::Error>>, io::Error>
    where R: io::Read
{
    let iter = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .has_headers(true)
        .from_reader(reader)
        .into_deserialize();

    Ok(iter)
}
