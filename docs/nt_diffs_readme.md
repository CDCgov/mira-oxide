# All Sample Hamming Distance

The all_sample_nt_diffs package takes a fasta file containing all your samples of interest after they have been aligned as an input. The outputs is a table containing all of the positions and nucleotide changes between all of the sequences within the fasta file provided.

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

After cloning the mira-oxide repo, execute this command to create the table of nucleotide differences and their positions for the samples provided:

```bash
 cargo run -p all_sample_nt_diff -- -i <PATH>/input.fasta -o <PATH>/outputs.csv
```

If you would like the output to have another deliminator (default: ","), then the `-d` flag can be used to pass another deliminator.

## The hamming distances output should be structured like this

```text
sequence_1,sequence_2,nt_sequence_1,positios,nt_sequence_2
sample-1-rep-1,sample-1-rep-2,C,34,T
sample-1-rep-1,sample-1-rep-3,A,115,C
sample-1-rep-1,sample-1-rep-3,A,116,T
sample-1-rep-2,sample-1-rep-1,T,34,C
sample-1-rep-2,sample-1-rep-3,T,34,C
sample-1-rep-2,sample-1-rep-3,A,115,C
sample-1-rep-2,sample-1-rep-3,A,116,T
sample-1-rep-3,sample-1-rep-1,C,115,A
sample-1-rep-3,sample-1-rep-1,T,116,A
sample-1-rep-3,sample-1-rep-2,C,34,T
sample-1-rep-3,sample-1-rep-2,C,115,A
sample-1-rep-3,sample-1-rep-2,T,116,A
```
