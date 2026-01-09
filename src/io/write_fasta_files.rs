use std::{error::Error, fs::File, io::Write, path::Path};

use crate::{io::data_ingest::SeqData, utils::data_processing::NextcladeSequences};

//////////////// Function to write fasta ///////////////
pub fn write_to_fasta(output_file: &str, seq_data_vec: &[SeqData]) -> Result<(), Box<dyn Error>> {
    let mut file = File::create(output_file)?;

    for seq_data in seq_data_vec {
        writeln!(file, ">{}", seq_data.name)?;
        writeln!(file, "{}", seq_data.sequence)?;
    }

    println!(" -> FASTA written to {output_file}");

    Ok(())
}

//////////////// Function to collection and write out all consensus fasta files ///////////////
pub fn write_out_all_consensus_fasta_files(
    output_path: &Path,
    nt_passed_vec: &[SeqData],
    nt_fail_vec: &[SeqData],
    aa_passed_vec: &[SeqData],
    aa_fail_vec: &[SeqData],
    runid: &str,
) -> Result<(), Box<dyn Error>> {
    let _ = write_to_fasta(
        &format!(
            "{}/mira_{runid}_amended_consensus.fasta",
            output_path.display()
        ),
        nt_passed_vec,
    );

    let _ = write_to_fasta(
        &format!(
            "{}/mira_{runid}_failed_amended_consensus.fasta",
            output_path.display()
        ),
        nt_fail_vec,
    );

    let _ = write_to_fasta(
        &format!(
            "{}/mira_{runid}_amino_acid_consensus.fasta",
            output_path.display()
        ),
        aa_passed_vec,
    );

    let _ = write_to_fasta(
        &format!(
            "{}/mira_{runid}_failed_amino_acid_consensus.fasta",
            output_path.display()
        ),
        aa_fail_vec,
    );

    Ok(())
}

pub fn write_out_nextclade_fasta_files(
    output_path: &Path,
    nextclade_seqs: &NextcladeSequences,
    runid: &str,
) -> Result<(), Box<dyn Error>> {
    if !nextclade_seqs.influenza_a_h3n2_ha.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_flu_h3n2_ha.fasta",
                output_path.display()
            ),
            &nextclade_seqs.influenza_a_h3n2_ha,
        )?;
    }

    if !nextclade_seqs.influenza_a_h1n1pdm_ha.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_flu_h1n1pdm_ha.fasta",
                output_path.display()
            ),
            &nextclade_seqs.influenza_a_h1n1pdm_ha,
        )?;
    }

    if !nextclade_seqs.influenza_b_victoria_ha.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_flu_vic_ha.fasta",
                output_path.display()
            ),
            &nextclade_seqs.influenza_b_victoria_ha,
        )?;
    }

    if !nextclade_seqs.influenza_a_h1n1pdm_na.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_flu_h1n1pdm_na.fasta",
                output_path.display()
            ),
            &nextclade_seqs.influenza_a_h1n1pdm_na,
        )?;
    }

    if !nextclade_seqs.influenza_a_h3n2_na.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_flu_h3n2_na.fasta",
                output_path.display()
            ),
            &nextclade_seqs.influenza_a_h3n2_na,
        )?;
    }

    if !nextclade_seqs.influenza_b_victoria_na.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_flu_vic_na.fasta",
                output_path.display()
            ),
            &nextclade_seqs.influenza_b_victoria_na,
        )?;
    }

    if !nextclade_seqs.rsv_a.is_empty() {
        write_to_fasta(
            &format!("{}/nextclade_{runid}_rsv_a.fasta", output_path.display()),
            &nextclade_seqs.rsv_a,
        )?;
    }

    if !nextclade_seqs.rsv_b.is_empty() {
        write_to_fasta(
            &format!("{}/nextclade_{runid}_rsv_b.fasta", output_path.display()),
            &nextclade_seqs.rsv_b,
        )?;
    }

    if !nextclade_seqs.sars_cov_2.is_empty() {
        write_to_fasta(
            &format!(
                "{}/nextclade_{runid}_sars-cov-2.fasta",
                output_path.display()
            ),
            &nextclade_seqs.sars_cov_2,
        )?;
    }

    Ok(())
}
