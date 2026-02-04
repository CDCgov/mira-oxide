use zoe::{
    alignment::{ScalarProfile, sw::sw_scalar_align},
    data::{ByteIndexMap, WeightMatrix},
};

#[must_use]
pub fn align_sequences<'a>(query: &'a [u8], reference: &'a [u8]) -> (Vec<u8>, Vec<u8>) {
    const MAPPING: ByteIndexMap<6> = ByteIndexMap::new(*b"ACGTN*", b'N');
    const WEIGHTS: WeightMatrix<i8, 6> = WeightMatrix::new(&MAPPING, 1, 0, Some(b'N'));
    const GAP_OPEN: i8 = -1;
    const GAP_EXTEND: i8 = 0;

    let profile = ScalarProfile::<6>::new(query, &WEIGHTS, GAP_OPEN, GAP_EXTEND)
        .expect("Alignment profile failed");
    let alignment = sw_scalar_align(reference, &profile);
    let alignment = match alignment {
        zoe::alignment::MaybeAligned::Some(alignment) => alignment,
        zoe::alignment::MaybeAligned::Overflowed => {
            unreachable!("Overflow will not occur in scalar alignments")
        }
        zoe::alignment::MaybeAligned::Unmapped => {
            return (Vec::new(), Vec::new());
        }
    };

    alignment.get_aligned_seqs(reference, query)
}
