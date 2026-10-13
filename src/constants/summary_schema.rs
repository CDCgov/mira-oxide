/// A field included in the initial MIRA summary outputs.
#[derive(Clone, Copy)]
pub struct SummaryColumn {
    /// Serialized field name on [`IRMASummary`](crate::utils::data_processing::IRMASummary).
    pub field: &'static str,
    /// Column name used by machine-readable outputs.
    pub output_name: &'static str,
    /// Human-readable heading used by the static HTML report.
    pub display_name: &'static str,
}

const DEFAULT_SUMMARY_SCHEMA: &[SummaryColumn] = &[
    SummaryColumn {
        field: "sample_id",
        output_name: "sample_id",
        display_name: "Sample",
    },
    SummaryColumn {
        field: "total_reads",
        output_name: "total_reads",
        display_name: "Total Reads",
    },
    SummaryColumn {
        field: "pass_qc",
        output_name: "pass_qc",
        display_name: "Pass QC",
    },
    SummaryColumn {
        field: "reads_mapped",
        output_name: "reads_mapped",
        display_name: "Reads Mapped",
    },
    SummaryColumn {
        field: "reference",
        output_name: "reference",
        display_name: "Reference",
    },
    SummaryColumn {
        field: "percent_reference_coverage",
        output_name: "percent_reference_coverage",
        display_name: "% Reference Covered",
    },
    SummaryColumn {
        field: "median_coverage",
        output_name: "median_coverage",
        display_name: "Median Coverage",
    },
    SummaryColumn {
        field: "count_minor_snv_at_or_over_5_pct",
        output_name: "count_minor_snv_at_or_over_5_pct",
        display_name: "Count of Minor SNVs >= 0.05",
    },
    SummaryColumn {
        field: "pass_fail_reason",
        output_name: "pass_fail_reason",
        display_name: "Pass/Fail Reason",
    },
    SummaryColumn {
        field: "subtype",
        output_name: "subtype",
        display_name: "Subtype",
    },
    SummaryColumn {
        field: "mira_module",
        output_name: "mira_version;module;irma_config",
        display_name: "MIRA module",
    },
    SummaryColumn {
        field: "runid",
        output_name: "runid",
        display_name: "Run ID",
    },
    SummaryColumn {
        field: "instrument",
        output_name: "instrument",
        display_name: "Instrument",
    },
];

const WGS_SUMMARY_SCHEMA: &[SummaryColumn] = &[
    SummaryColumn {
        field: "sample_id",
        output_name: "sample_id",
        display_name: "Sample",
    },
    SummaryColumn {
        field: "total_reads",
        output_name: "total_reads",
        display_name: "Total Reads",
    },
    SummaryColumn {
        field: "pass_qc",
        output_name: "pass_qc",
        display_name: "Pass QC",
    },
    SummaryColumn {
        field: "reads_mapped",
        output_name: "reads_mapped",
        display_name: "Reads Mapped",
    },
    SummaryColumn {
        field: "reference",
        output_name: "reference",
        display_name: "Reference",
    },
    SummaryColumn {
        field: "percent_reference_coverage",
        output_name: "percent_reference_coverage",
        display_name: "% Reference Covered",
    },
    SummaryColumn {
        field: "median_coverage",
        output_name: "median_coverage",
        display_name: "Median Coverage",
    },
    SummaryColumn {
        field: "count_minor_snv_at_or_over_5_pct",
        output_name: "count_minor_snv_at_or_over_5_pct",
        display_name: "Count of Minor SNVs >= 0.05",
    },
    SummaryColumn {
        field: "spike_percent_coverage",
        output_name: "spike_percent_coverage",
        display_name: "Spike % Coverage",
    },
    SummaryColumn {
        field: "spike_median_coverage",
        output_name: "spike_median_coverage",
        display_name: "Spike Median Coverage",
    },
    SummaryColumn {
        field: "pass_fail_reason",
        output_name: "pass_fail_reason",
        display_name: "Pass/Fail Reason",
    },
    SummaryColumn {
        field: "subtype",
        output_name: "subtype",
        display_name: "Subtype",
    },
    SummaryColumn {
        field: "mira_module",
        output_name: "mira_version;module;irma_config",
        display_name: "MIRA module",
    },
    SummaryColumn {
        field: "runid",
        output_name: "runid",
        display_name: "Run ID",
    },
    SummaryColumn {
        field: "instrument",
        output_name: "instrument",
        display_name: "Instrument",
    },
];

const FLU_SUMMARY_SCHEMA: &[SummaryColumn] = &[
    SummaryColumn {
        field: "sample_id",
        output_name: "sample_id",
        display_name: "Sample",
    },
    SummaryColumn {
        field: "total_reads",
        output_name: "total_reads",
        display_name: "Total Reads",
    },
    SummaryColumn {
        field: "pass_qc",
        output_name: "pass_qc",
        display_name: "Pass QC",
    },
    SummaryColumn {
        field: "reads_mapped",
        output_name: "reads_mapped",
        display_name: "Reads Mapped",
    },
    SummaryColumn {
        field: "reference",
        output_name: "reference",
        display_name: "Reference",
    },
    SummaryColumn {
        field: "percent_reference_coverage",
        output_name: "percent_reference_coverage",
        display_name: "% Reference Covered",
    },
    SummaryColumn {
        field: "median_coverage",
        output_name: "median_coverage",
        display_name: "Median Coverage",
    },
    SummaryColumn {
        field: "count_minor_snv_at_or_over_5_pct",
        output_name: "count_minor_snv_at_or_over_5_pct",
        display_name: "Count of Minor SNVs >= 0.05",
    },
    SummaryColumn {
        field: "di_ratios_5prime_3prime",
        output_name: "di_5prime;di_3prime",
        display_name: "DI Ratios 5'/3'",
    },
    SummaryColumn {
        field: "secondary_hits",
        output_name: "secondary_hits",
        display_name: "Secondary Hits",
    },
    SummaryColumn {
        field: "pass_fail_reason",
        output_name: "pass_fail_reason",
        display_name: "Pass/Fail Reason",
    },
    SummaryColumn {
        field: "subtype",
        output_name: "subtype",
        display_name: "Subtype",
    },
    SummaryColumn {
        field: "mira_module",
        output_name: "mira_version;module;irma_config",
        display_name: "MIRA module",
    },
    SummaryColumn {
        field: "runid",
        output_name: "runid",
        display_name: "Run ID",
    },
    SummaryColumn {
        field: "instrument",
        output_name: "instrument",
        display_name: "Instrument",
    },
];

/// Returns the initial summary schema for a virus.
#[must_use]
pub fn summary_schema(virus: &str) -> &'static [SummaryColumn] {
    match virus {
        "sc2-wgs" => WGS_SUMMARY_SCHEMA,
        "flu" => FLU_SUMMARY_SCHEMA,
        _ => DEFAULT_SUMMARY_SCHEMA,
    }
}
