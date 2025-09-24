# All Sample Hamming Distance

The all_sample_nt_diffs package takes a fasta file containing all your samples of interest after they have been aligned as an input. The outputs is a hamming distance matrix that provides the hamming distance between all of the sequences within the fasta file provided.

## FASTA file Input

```fasta
>sample-1-rep-1
ATGGAGAGAATAAAAGAACTGAGAGATCTAATGTCACAGTCTCGCACTCGCGAGATACTA
ACCAAAACCACTGTTGACCACATGGCCATAATCAAGAAGTACACATCAGGAAGACAAGAA
>sample-1-rep-2
ATGGAGAGAATAAAAGAACTGAGAGATCTAATGTTACAGTCTCGCACTCGCGAGATACTA
ACCAAAACCACTGTTGACCACATGGCCATAATCAAGAAGTACACATCAGGAAGACAAGAA
>sample-1-rep-3
ATGGAGAGAATAAAAGAACTGAGAGATCTAATGTCACAGTCTCGCACTCGCGAGATACTA
ACCAAAACCACTGTTGACCACATGGCCATAATCAAGAAGTACACATCAGGAAGACCTGAA
```

After cloning the mira-oxide repo, execute this command to create a hamming distance matrix for the samples provided:

```bash
 cargo run -p all_sample_hamming_dist -- -i <PATH>/input.fasta -o <PATH>/outputs.csv
```

If you would like the output to have another deliminator (default: ","), then the `-d` flag can be used to pass another deliminator.

## The hamming distances output should be structured like this

```text
seqeunces,sample-1-rep-1,sample-1-rep-2,sample-1-rep-3
sample-1-rep-1, 0, 1, 2
sample-1-rep-2, 1, 0, 3
sample-1-rep-3, 2, 3, 0
```
