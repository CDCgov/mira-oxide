pub const FLU_PROTEINS: &[(&str, &[&str])] = &[
    ("HA", &["HA1", "HA", "HA-signal"]),
    ("NA", &["NA", "NB"]),
    ("MP", &["M1", "M2", "BM2"]),
    ("NP", &["NP"]),
    ("NS", &["NS1", "NS2", "NEP"]),
    ("PA", &["PA", "PA-X"]),
    ("PB1", &["PB1", "PB1-F2"]),
    ("PB2", &["PB2"]),
];

pub const SC2_PROTEINS: &[(&str, &[&str])] = &[(
    "SARS-CoV-2",
    &[
        "orf1ab", "ORF7b", "ORF9b", "ORF10", "ORF8", "ORF7b", "ORF6", "ORF3a", "S", "N", "M", "E",
    ],
)];
