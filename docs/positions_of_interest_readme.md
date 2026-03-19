# Positions of Interest Package

The positions of interest package takes a DAIS-ribosome output from your samples of interest, a reference table with sequences that have been aligned using DAIS-ribosome and a table containing the amino acid mutations of interest as inputs. The output is a list of all samples with codon with aa and codon infromation whether they had nucleotide variations or not at the specificed positions when compared to the reference. 

## DAIS-ribosome output of samples should be structured like this (tab delimited)

```text
s3_4   A_HA_H1   CALI07   HA-signal   e414bb92673b4dcbd44904630366ab9f   MKAILVVLLYTFTTANA   MKAILVVLLYTFTTANA   b7f1bb79d3fc87c1eb076af599c4f99e8fa7d183   false   false   ATGAAGGCAATACTAGTAGTTCTGCTGTATACATTTACAACCGCAAATGCA   ATGAAGGCAATACTAGTAGTTCTGCTGTATACATTTACAACCGCAAATGCA   1..51   1..51
s1_4   A_HA_H3   HK4801   HA-signal   abd16f5208e6affe54d06d3ba7365f6e   MKAIIALSNILCLVFA   MKAIIALSNILCLVFA   0456cc08adaf88e3a88469276ac3b69b9278dfaf   false   false   ATGAAGGCTATCATTGCTTTGAGCAACATTCTATGTCTTGTTTTCGCT   ATGAAGGCTATCATTGCTTTGAGCAACATTCTATGTCTTGTTTTCGCT   1..48   1..48
s1_7   A_MP   HK4801   M2   feacaf2e5966faa102eea35f5145a657   MSLLTEVETPIRNEWGCRCNDSSNPLVVAANIIGILHLILWILDRLFFKCVYRLFKHGLKRGPSTEGVPESMREEYRKEQQNAVDADESHFVSIELE*   MSLLTEVETPIRNEWGCRCNDSSNPLVVAANIIGILHLILWILDRLFFKCVYRLFKHGLKRGPSTEGVPESMREEYRKEQQNAVDADESHFVSIELE*   332c058b16e9b4194021980f77c4a50c2090e8ec   false   false   ATGAGCCTTCTTACCGAGGTCGAAACACCTATCAGAAACGAATGGGGGTGCAGATGCAACGATTCAAGTAATCCACTTGTTGTTGCCGCGAATATCATTGGGATCTTGCACTTGATATTATGGATTCTTGATCGTCTTTTTTTCAAATGCGTCTATCGACTCTTCAAACACGGCCTTAAAAGAGGCCCTTCTACGGAAGGTGTGCCTGAGTCTATGAGGGAAGAATACCGAAAGGAACAGCAGAATGCTGTGGATGCTGACGAAAGTCATTTTGTCAGCATAGAATTGGAGTAA   ATGAGCCTTCTTACCGAGGTCGAAACACCTATCAGAAACGAATGGGGGTGCAGATGCAACGATTCAAGTAATCCACTTGTTGTTGCCGCGAATATCATTGGGATCTTGCACTTGATATTATGGATTCTTGATCGTCTTTTTTTCAAATGCGTCTATCGACTCTTCAAACACGGCCTTAAAAGAGGCCCTTCTACGGAAGGTGTGCCTGAGTCTATGAGGGAAGAATACCGAAAGGAACAGCAGAATGCTGTGGATGCTGACGAAAGTCATTTTGTCAGCATAGAATTGGAGTAA   1..26;715..982   1..26;27..294
```

## The Reference Table input should be structured like this (tab delimited)

```text
isolate_id   isolate_name   subtype   passage_history   nt_id   ctype   reference_id   protein   aa_aln   cds_aln
EPI_ISL_25690   A/common magpie/Hong Kong/5052/2007   A / H5N1      2b14fd2e8f738834298e9099f00e59d020ffc552   A_HA_H5   VT1203   HA-signal   .....LLFAIVSLVKS   ...............CTTCTTTTTGCAATAGTCAGCCTTGTTAAAAGC
EPI_ISL_140   A/Hong Kong/1073/99   A / H9N2      a591bc9ad3a54f705940ad8483684cfc278c742c   A_HA_H9   BGD0994   HA-signal   METISLITILLVVTASNA   ATGGAAACAATATCACTAATAACTATACTACTAGTAGTAACAGCAAGCAATGCA
```

### The Known Positions of Interest Table input should be structured like this (tab delimited)

```text
subtype	protein	positions	amino_acid	phenotypic_consequence
A / H1N1	NA	275	Y	true
A / H1N1	NA	223	X	true
A / H1N1	NA	119	V	true
A / H1N1	NA	292	K	true
A / H1N1	NA	294	S	true
A / H1N1	NA	197	N	true
A / H1N1	PA	38	T	true
A / H3N2	NA	119	V	true
B	NA	197	N	true
B	PA	38	T	true
```

After cloning the mira-oxide repo, execute this command to create a positions of interest table for the samples:

```bash
cargo run -p positions-of-interest -- -i <PATH>/DAIS_ribosome.seq -r <PATH>/ref_table.txt -o <PATH>/outputs.csv -m <PATH>/positions_of_interest.txt
```

If you would like the output to have another deliminator (default: ","), then the `-d` flag can be used to pass another deliminator.

### The Positions of Interest Table output should be structured like this (comma delimited)

```text
sample, reference_strain,gisaid_accession,ctype,dais_reference,protein,sample_codon,reference_codon,aa_mutation,phenotypic_consequence
sample_3_6,A/Georgia/12/2022,EPI_ISL_15724408,A_NA_N1,CALI07,NA,GAA,GAA,E:119:E,
sample_3_6,A/Georgia/12/2022,EPI_ISL_15724408,A_NA_N1,CALI07,NA,GGC,GGC,G:197:G,
sample_3_6,A/Georgia/12/2022,EPI_ISL_15724408,A_NA_N1,CALI07,NA,ATA,ATA,I:223:I,
sample_3_6,A/Georgia/12/2022,EPI_ISL_15724408,A_NA_N1,CALI07,NA,CAC,CAC,H:275:H,
sample_3_6,A/Georgia/12/2022,EPI_ISL_15724408,A_NA_N1,CALI07,NA,TGC,TGC,C:292:C,
sample_3_6,A/Georgia/12/2022,EPI_ISL_15724408,A_NA_N1,CALI07,NA,GAT,GAT,D:294:D,
sample_3_6,A/California/07/2009,EPI_ISL_227813,A_NA_N1,CALI07,NA,GAA,GAA,E:119:E,
sample_3_6,A/California/07/2009,EPI_ISL_227813,A_NA_N1,CALI07,NA,GGC,GGC,G:197:G,
sample_3_6,A/California/07/2009,EPI_ISL_227813,A_NA_N1,CALI07,NA,ATA,ATA,I:223:I,
sample_3_6,A/California/07/2009,EPI_ISL_227813,A_NA_N1,CALI07,NA,CAC,CAC,H:275:H,
sample_3_6,A/California/07/2009,EPI_ISL_227813,A_NA_N1,CALI07,NA,TGC,TGC,C:292:C,
sample_3_6,A/California/07/2009,EPI_ISL_227813,A_NA_N1,CALI07,NA,GAT,GAT,D:294:D,
sample_3_6,A/Wisconsin/67/2022,EPI_ISL_15928538,A_NA_N1,CALI07,NA,GAA,GAA,E:119:E,
sample_3_6,A/Wisconsin/67/2022,EPI_ISL_15928538,A_NA_N1,CALI07,NA,GGC,GGC,G:197:G,
sample_3_6,A/Wisconsin/67/2022,EPI_ISL_15928538,A_NA_N1,CALI07,NA,ATA,ATA,I:223:I,
sample_3_6,A/Wisconsin/67/2022,EPI_ISL_15928538,A_NA_N1,CALI07,NA,CAC,CAC,H:275:H,
sample_3_6,A/Wisconsin/67/2022,EPI_ISL_15928538,A_NA_N1,CALI07,NA,TGC,TGC,C:292:C,
sample_3_6,A/Wisconsin/67/2022,EPI_ISL_15928538,A_NA_N1,CALI07,NA,GAT,GAT,D:294:D,
sample_3_6,A/West Virginia/30/2022,EPI_ISL_15724406,A_NA_N1,CALI07,NA,GAA,GAA,E:119:E,
sample_3_6,A/West Virginia/30/2022,EPI_ISL_15724406,A_NA_N1,CALI07,NA,GGC,GGC,G:197:G,
sample_3_6,A/West Virginia/30/2022,EPI_ISL_15724406,A_NA_N1,CALI07,NA,ATA,ATA,I:223:I,
sample_3_6,A/West Virginia/30/2022,EPI_ISL_15724406,A_NA_N1,CALI07,NA,CAC,CAC,H:275:H,
sample_3_6,A/West Virginia/30/2022,EPI_ISL_15724406,A_NA_N1,CALI07,NA,TGC,TGC,C:292:C,
sample_3_6,A/West Virginia/30/2022,EPI_ISL_15724406,A_NA_N1,CALI07,NA,GAT,GAT,D:294:D,
sample_4_6,A/California/45/2023,EPI_ISL_17625824,A_NA_N2,HK4801,NA,GAA,GAA,E:119:E,
sample_4_6,A/Ohio/28/2016,EPI_ISL_232045,A_NA_N2,HK4801,NA,GAA,GAA,E:119:E,
sample_4_3,A/California/07/2009,EPI_ISL_227813,A_PA,HK4801,PA,ATA,ATA,I:38:I,
sample_4_3,A/Georgia/12/2022,EPI_ISL_15724408,A_PA,HK4801,PA,ATT,ATA,I:38:I,
sample_4_3,A/Wisconsin/67/2022,EPI_ISL_15928538,A_PA,HK4801,PA,ATT,ATA,I:38:I,
sample_4_3,A/West Virginia/30/2022,EPI_ISL_15724406,A_PA,HK4801,PA,ATT,ATA,I:38:I,
sample_3_3,A/California/07/2009,EPI_ISL_227813,A_PA,HK4801,PA,ATA,ATT,I:38:I,
sample_3_3,A/Georgia/12/2022,EPI_ISL_15724408,A_PA,HK4801,PA,ATT,ATT,I:38:I,
sample_3_3,A/Wisconsin/67/2022,EPI_ISL_15928538,A_PA,HK4801,PA,ATT,ATT,I:38:I,
sample_3_3,A/West Virginia/30/2022,EPI_ISL_15724406,A_PA,HK4801,PA,ATT,ATT,I:38:I,
sample_1_6,B/Connecticut/01/2021,EPI_ISL_3856740,B_NA,PHUKET3073,NA,GAC,GAC,D:197:D,
sample_1_3,B/Connecticut/01/2021,EPI_ISL_3856740,B_PA,PHUKET3073,PA,ATC,ATC,I:38:I,
```

----------------------------------------------------------------------------------

## Positions of Interest Package Version with Minor Variants

![Alt text](../assets/images/coming_soon.png)
