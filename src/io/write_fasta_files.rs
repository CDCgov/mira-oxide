use std::{error::Error, fs::File, io::Write, path::Path};

use crate::io::data_ingest::SeqData;

//////////////// Function to write fasta ///////////////
pub fn write_to_fasta(output_file: &str, seq_data_vec: &[SeqData]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(output_file)?;

    for seq_data in seq_data_vec {
        writeln!(file, ">{}", seq_data.name)?;
        writeln!(file, "{}", seq_data.sequence)?;
    }

    println!("FASTA written to {output_file}");

    Ok(())
}

//////////////// Function to collection and write out all fasta files ///////////////
pub fn write_out_all_fasta_files(
    output_path: &Path,
    nt_passed_vec: &[SeqData],
    nt_fail_vec: &[SeqData],
    aa_passed_vec: &[SeqData],
    aa_fail_vec: &[SeqData],
    runid: &str,
) -> Result<(), Box<dyn Error>> {
    let _ = write_to_fasta(
        &format!(
            "{}/{runid}_amended_consensus_summary.fasta",
            output_path.display()
        ),
        nt_passed_vec,
    );

    let _ = write_to_fasta(
        &format!(
            "{}/{runid}_failed_amended_consensus_summary.fasta",
            output_path.display()
        ),
        nt_fail_vec,
    );

    let _ = write_to_fasta(
        &format!(
            "{}/{runid}_amino_acid_consensus_summary.fasta",
            output_path.display()
        ),
        aa_passed_vec,
    );

    let _ = write_to_fasta(
        &format!(
            "{}/{runid}_failed_amino_acid_consensus_summary.fasta",
            output_path.display()
        ),
        aa_fail_vec,
    );

    Ok(())
}
