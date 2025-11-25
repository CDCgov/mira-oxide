use clap::Parser;
use std::{
    fs,
    io::{self, ErrorKind},
    path::PathBuf,
};

#[derive(Debug, Parser)]
#[command(about = "Get relevant IRMA configuration and modules for the current experiment.")]
pub struct MiraVersionArgs {
    #[arg(short = 'g', long)]
    /// Github url to description file on MIRA-NF github
    pub git_version: String,

    #[arg(short = 'l', long)]
    /// path to local description file
    pub local_version_path: PathBuf,
}

fn extract_version_line(contents: &str) -> Option<&str> {
    contents.lines().find(|line| line.starts_with("Version"))
}

pub fn check_mira_version(args: MiraVersionArgs) -> io::Result<()> {
    let local_desc_path = args.local_version_path.join("DESCRIPTION");
    let local_contents = fs::read_to_string(&local_desc_path)?;

    let git_contents = fs::read_to_string(&args.git_version)?;

    let current = extract_version_line(&local_contents).ok_or_else(|| {
        io::Error::new(
            ErrorKind::InvalidData,
            "No Version line in local DESCRIPTION",
        )
    })?;

    let available = extract_version_line(&git_contents).ok_or_else(|| {
        io::Error::new(ErrorKind::InvalidData, "No Version line in git DESCRIPTION")
    })?;

    if current >= available {
        println!("MIRA-NF version up to date!");
    } else {
        println!("MIRA-NF {} is now available!", available);
    }

    Ok(())
}
