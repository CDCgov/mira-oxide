use crate::processes::prepare_mira_reports::Virus;

// Refs for Flu heatmaps
pub const FLU_SEGMENTS: [&str; 8] = ["PB1", "PB2", "PA", "HA", "NP", "NA", "MP", "NS"];
// Refs for SARS-CoV-2 heatmaps
pub const SC2_GENOME: &str = "SARS-CoV-2";
// Refs for RSV heatmaps
pub const RSV_GENOME: &str = "RSV";

// Function to obtain reference based on virus
pub fn get_references_for_virus(virus: Virus) -> Vec<String> {
    match virus {
        Virus::Flu => FLU_SEGMENTS.iter().map(ToString::to_string).collect(),
        Virus::Sc2Wgs | Virus::Sc2Spike => vec![SC2_GENOME.to_string()],
        Virus::Rsv => vec![RSV_GENOME.to_string()],
    }
}
