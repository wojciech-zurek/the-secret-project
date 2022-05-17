use crate::CliError;

pub fn write<T>(iter: impl Iterator<Item=T>) -> Result<(), CliError>
    where T: serde::Serialize
{
    //we can easy change writer to other types
    write_csv(std::io::stdout(), iter)?;
    Ok(())
}

pub fn write_csv<W, T>(writer: W, iter: impl Iterator<Item=T>) -> Result<(), csv::Error>
    where
        W: std::io::Write,
        T: serde::Serialize,
{
    let mut writer = csv::WriterBuilder::new()
        .has_headers(true)
        .from_writer(writer);

    for record in iter {
        writer.serialize(record)?;
    }

    writer.flush()?;

    Ok(())
}