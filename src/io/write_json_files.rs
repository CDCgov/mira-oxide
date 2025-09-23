//////////////// Function to collection and write out all CSV files ///////////////

pub fn write_out_all_fasta_files(
    nt_passed_vec: &[SeqData],
    nt_fail_vec: &[SeqData],
    aa_passed_vec: &[SeqData],
    aa_fail_vec: &[SeqData],
    runid: &str,
) -> Result<(), Box<dyn Error>> {
    // Writing out Coverage data
    let coverage_struct_values = vec![
        "Sample",
        "Reference_Name",
        "Position",
        "Coverage Depth",
        "Consensus",
        "Deletions",
        "Ambiguous",
        "Consensus_Count",
        "Consensus_Average_Quality",
    ];

    let coverage_columns = vec![
        "sample_id",
        "reference",
        "reference_position",
        "depth",
        "consensus",
        "deletions",
        "ambiguous",
        "consensus_count",
        "consensus_quality",
    ];

    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/coverage_data.json",
        &coverage_data,
        &coverage_columns,
        &coverage_struct_values,
    )?;

    // Writing out reads data
    let reads_struct_values = vec![
        "Sample",
        "Record",
        "Reads",
        "Patterns",
        "PairsAndWidows",
        "Stage",
    ];
    let reads_columns = vec![
        "sample_id",
        "record",
        "reads",
        "patterns",
        "pairs_and_windows",
        "stage",
    ];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/reads.json",
        &read_data,
        &reads_columns,
        &reads_struct_values,
    )?;

    // Writing out vtype data (json only)
    let vtype_columns = vec!["sample_id", "vtype", "ref_type", "subtype"];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/vtype.json",
        &vtype_data,
        &vtype_columns,
        &vtype_columns,
    )?;

    // Writing out allele \
    let allele_struct_values = vec![
        "Sample",
        "Upstream_Position",
        "Reference_Name",
        "Context",
        "Length",
        "Insert",
        "Count",
        "Total",
        "Frequency",
    ];
    let allele_columns = vec![
        "sample",
        "sample_upstream_position",
        "reference",
        "context",
        "length",
        "insert",
        "count",
        "upstream_base_coverage",
        "frequency",
    ];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/alleles.json",
        &allele_data,
        &allele_columns,
        &allele_struct_values,
    )?;

    // Writing out indel
    let indels_struct_values = vec![
        "Sample",
        "Upstream_Position",
        "Reference_Name",
        "Context",
        "Length",
        "Insert",
        "Count",
        "Total",
        "Frequency",
    ];
    let indels_columns = vec![
        "sample",
        "sample_upstream_position",
        "reference",
        "context",
        "length",
        "insert",
        "count",
        "upstream_base_coverage",
        "frequency",
    ];
    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/indels.json",
        &indel_data,
        &indels_columns,
        &indels_struct_values,
    )?;

    // Write out ref_data.json
    write_ref_data_json(
        "/home/xpa3/mira-oxide/test/ref_data.json",
        &ref_lengths,
        &segments,
        &segset,
        &segcolor,
    )?;

    // write out the dais_vars.json
    let aavars_columns = vec![
        "sample_id",
        "reference_id",
        "protein",
        "aa_variant_count",
        "aa_variants",
    ];

    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/dais_vars.json",
        &dais_vars_data,
        &aavars_columns,
        &aavars_columns,
    )?;

    // write out the summary.json and the {runid}_summary.csv
    let summary_columns: Vec<&str> = if args.virus.to_lowercase() == "sc2-wgs" {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "precent_reference_coverage",
            "median_coverage",
            "count_minor_snv",
            "count_minor_indel",
            "spike_percent_coverage",
            "spike_median_coverage",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
        ]
    } else {
        vec![
            "sample_id",
            "total_reads",
            "pass_qc",
            "reads_mapped",
            "reference",
            "precent_reference_coverage",
            "median_coverage",
            "count_minor_snv",
            "count_minor_indel",
            "pass_fail_reason",
            "subtype",
            "mira_module",
            "runid",
            "instrument",
        ]
    };

    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/irma_summary.json",
        &irma_summary,
        &summary_columns,
        &summary_columns,
    )?;

    write_irma_summary_to_pass_fail_json_file(
        "/home/xpa3/mira-oxide/test/pass_fail_qc.json",
        &irma_summary,
    )?;

    // write out the nt_sequences.json
    let nt_seq_columns: Vec<&str> = if args.virus.to_lowercase() == "flu" {
        vec!["sample_id", "sequence", "target_ref", "reference"]
    } else {
        vec!["sample_id", "sequence", "reference"]
    };

    write_structs_to_split_json_file(
        "/home/xpa3/mira-oxide/test/nt_sequences.json",
        &nt_seq_vec,
        &nt_seq_columns,
        &nt_seq_columns,
    )?;
}
