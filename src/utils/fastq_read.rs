use flate2::read::MultiGzDecoder;
use std::{fs::File, io::Read, path::Path};
use zoe::{define_whichever, prelude::FastQReader};

define_whichever! {
    #[doc="An enum for the different acceptable input types"]
    pub(crate) enum ReadFileZip {
        #[doc="A reader for a regular uncompressed file"]
        File(File),
        #[doc="A reader for a gzip compressed file"]
        Zipped(MultiGzDecoder<File>),
    }

    impl Read for ReadFileZip {}
}

/// If the filename ends in `gz`, the file is assumed to be zipped.
///
/// ## Errors
///
/// `path` must exist and contain FASTQ data.
pub(crate) fn is_gz<P: AsRef<Path>>(path: P) -> bool {
    path.as_ref().extension().is_some_and(|ext| ext == "gz")
}

/// Open a single FASTQ file.
///
/// If it ends in `gz`, returns a [`FastQReader`] backed by
/// [`ReadFileZip::Zipped`], otherwise, returns a [`FastQReader`] backed by
/// [`ReadFileZip::File`].
#[inline]
pub(crate) fn open_fastq_file<P: AsRef<Path>>(
    path: P,
) -> std::io::Result<FastQReader<ReadFileZip>> {
    let file = File::open(&path)?;

    if is_gz(&path) {
        Ok(FastQReader::from_readable(ReadFileZip::Zipped(
            MultiGzDecoder::new(file),
        ))?)
    } else {
        Ok(FastQReader::from_readable(ReadFileZip::File(file))?)
    }
}
