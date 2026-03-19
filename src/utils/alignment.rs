use zoe::{
    alignment::{LocalProfiles, MaybeAligned},
    data::{ByteIndexMap, WeightMatrix},
    prelude::{ProfileSets, SeqSrc},
};

// note, in the future, we may want to revise this function to take a pre-built
// profile, since the outer loop calling this function in variants_of_interest

#[must_use]
pub fn align_sequences<'a>(query: &'a [u8], reference: &'a [u8]) -> (Vec<u8>, Vec<u8>) {
    const MAPPING: ByteIndexMap<6> = ByteIndexMap::new(*b"ACGTN*", b'N');
    const WEIGHTS: WeightMatrix<i8, 6> = WeightMatrix::new(&MAPPING, 1, 0, Some(b'N'));
    const GAP_OPEN: i8 = -1;
    const GAP_EXTEND: i8 = 0;

    let profile = LocalProfiles::new_with_w256(query, &WEIGHTS, GAP_OPEN, GAP_EXTEND)
        .expect("Alignment profile failed");
    let alignment = profile.sw_align_from_i8(SeqSrc::Reference(reference));
    let alignment = match alignment {
        MaybeAligned::Some(alignment) => alignment,
        MaybeAligned::Overflowed => {
            panic!("The alignment score has overflowed the capacity of an i32")
        }
        MaybeAligned::Unmapped => {
            return (Vec::new(), Vec::new());
        }
    };

    alignment.get_aligned_seqs(reference, query)
}
