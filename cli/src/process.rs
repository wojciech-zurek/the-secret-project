use clap::ArgMatches;
use crate::{CliError, ErrorType};
use core::TransactionProcessor;
use core::BasicProcessor;
use core::transaction::Transaction;
use crate::reader::read_from_file;
use crate::write::write;

pub fn execute(matches: &ArgMatches) -> Result<(), CliError> {
    let file_path = matches.value_of("file_path").ok_or_else(|| {
        // this should not happen at this stage
        CliError::new(ErrorType::CliParseError, "Arg file path not found")
    })?;

    // read file, deserialize csv via serde and return iterator
    let tx_iter = read_from_file(file_path)?;

    // use default process for transaction
    // we can easily create new one or use
    // use core::processor::advance_account_processor::LockAccountTransactionProcessor if we need lock mechanism
    let proc_iter = process(tx_iter, BasicProcessor::new())?;

    // Write csv and use stdout writer.
    // The output should be a list of client IDs (client), available amounts (available), held amounts
    // (held), total amounts (total), and whether the account is locked (locked).
    write(proc_iter)?;

    Ok(())
}

pub fn process<I, P, S>(iter: I, processor: P) -> Result<impl Iterator<Item=S>, CliError>
    where I: Iterator<Item=Result<Transaction, csv::Error>>,
          P: TransactionProcessor + IntoIterator<Item=S>,
          S: serde::Serialize

{
    let mut processor = processor;

    for record in iter {

        //check if record contains error
        //if yes - abort
        let transaction = record?;

        // send transaction for processing
        if let Err(e) = processor.process(transaction) {
            let _ = e;
            // in real world scenario we must do something with error case
            // we can put transaction with process error to dlq repository
            // or print to stderr or save to error file or do something else or do nothing (just kidding :) )
        }
    }

    Ok(processor.into_iter())
}