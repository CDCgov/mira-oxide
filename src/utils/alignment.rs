use zoe::{
    alignment::{MaybeAligned, ScalarProfile, sw::sw_scalar_alignment},
    data::{ByteIndexMap, WeightMatrix},
};

/// Take a query slice and reference slice and returns the same sequences back
/// in their optimal local alignment
#[must_use]
pub fn align_sequences<'a>(query: &'a [u8], reference: &'a [u8]) -> (Vec<u8>, Vec<u8>) {
    const MAPPING: ByteIndexMap<6> = ByteIndexMap::new(*b"ACGTN*", b'N');
    const WEIGHTS: WeightMatrix<i8, 6> = WeightMatrix::new(&MAPPING, 1, 0, Some(b'N'));
    const GAP_OPEN: i8 = -1;
    const GAP_EXTEND: i8 = 0;

    let profile = ScalarProfile::<6>::new(query, &WEIGHTS, GAP_OPEN, GAP_EXTEND)
        .expect("Alignment profile failed");
    let alignment = sw_scalar_alignment(reference, &profile);
    let alignment = match alignment {
        MaybeAligned::Some(alignment) => alignment,
        MaybeAligned::Overflowed => unreachable!("Scalar should not ever overflow"),
        MaybeAligned::Unmapped => {
            return (Vec::new(), Vec::new());
        }
    };

    alignment.get_aligned_seqs(reference, query)
}
