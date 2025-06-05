# Mutations of Interest Package

The mutations of interest package takes a DAIS-ribosome output from your samples of interest, a reference table with sequences that have been aligned using DAIS-ribosome and a table containing the amino acid mutations of interest as inputs. The output is a list of nucleotide and amino acid mutations that were found within the given samples when compared to the reference. 

### DAIS-ribosome output of samples should be structured like this (tab delimited):
```
s3_4	A_HA_H1	CALI07	HA-signal	e414bb92673b4dcbd44904630366ab9f	MKAILVVLLYTFTTANA	MKAILVVLLYTFTTANA	b7f1bb79d3fc87c1eb076af599c4f99e8fa7d183	false	false	ATGAAGGCAATACTAGTAGTTCTGCTGTATACATTTACAACCGCAAATGCA	ATGAAGGCAATACTAGTAGTTCTGCTGTATACATTTACAACCGCAAATGCA	1..51	1..51
s1_4	A_HA_H3	HK4801	HA-signal	abd16f5208e6affe54d06d3ba7365f6e	MKAIIALSNILCLVFA	MKAIIALSNILCLVFA	0456cc08adaf88e3a88469276ac3b69b9278dfaf	false	false	ATGAAGGCTATCATTGCTTTGAGCAACATTCTATGTCTTGTTTTCGCT	ATGAAGGCTATCATTGCTTTGAGCAACATTCTATGTCTTGTTTTCGCT	1..48	1..48
s1_7	A_MP	HK4801	M2	feacaf2e5966faa102eea35f5145a657	MSLLTEVETPIRNEWGCRCNDSSNPLVVAANIIGILHLILWILDRLFFKCVYRLFKHGLKRGPSTEGVPESMREEYRKEQQNAVDADESHFVSIELE*	MSLLTEVETPIRNEWGCRCNDSSNPLVVAANIIGILHLILWILDRLFFKCVYRLFKHGLKRGPSTEGVPESMREEYRKEQQNAVDADESHFVSIELE*	332c058b16e9b4194021980f77c4a50c2090e8ec	false	false	ATGAGCCTTCTTACCGAGGTCGAAACACCTATCAGAAACGAATGGGGGTGCAGATGCAACGATTCAAGTAATCCACTTGTTGTTGCCGCGAATATCATTGGGATCTTGCACTTGATATTATGGATTCTTGATCGTCTTTTTTTCAAATGCGTCTATCGACTCTTCAAACACGGCCTTAAAAGAGGCCCTTCTACGGAAGGTGTGCCTGAGTCTATGAGGGAAGAATACCGAAAGGAACAGCAGAATGCTGTGGATGCTGACGAAAGTCATTTTGTCAGCATAGAATTGGAGTAA	ATGAGCCTTCTTACCGAGGTCGAAACACCTATCAGAAACGAATGGGGGTGCAGATGCAACGATTCAAGTAATCCACTTGTTGTTGCCGCGAATATCATTGGGATCTTGCACTTGATATTATGGATTCTTGATCGTCTTTTTTTCAAATGCGTCTATCGACTCTTCAAACACGGCCTTAAAAGAGGCCCTTCTACGGAAGGTGTGCCTGAGTCTATGAGGGAAGAATACCGAAAGGAACAGCAGAATGCTGTGGATGCTGACGAAAGTCATTTTGTCAGCATAGAATTGGAGTAA	1..26;715..982	1..26;27..294
```

### The Reference Table input should be structured like this (tab delimited):
```
isolate_id	isolate_name	subtype	passage_history	nt_id	ctype	reference_id	protein	aa_aln	cds_aln
EPI_ISL_25690	A/common magpie/Hong Kong/5052/2007	A / H5N1		2b14fd2e8f738834298e9099f00e59d020ffc552	A_HA_H5	VT1203	HA-signal	.....LLFAIVSLVKS	...............CTTCTTTTTGCAATAGTCAGCCTTGTTAAAAGC
EPI_ISL_140	A/Hong Kong/1073/99	A / H9N2		a591bc9ad3a54f705940ad8483684cfc278c742c	A_HA_H9	BGD0994	HA-signal	METISLITILLVVTASNA	ATGGAAACAATATCACTAATAACTATACTACTAGTAGTAACAGCAAGCAATGCA

```

### The Known Mutations of Interest Table input should be structured like this (tab delimited):

```
protein position    mutation_of_int phenotypic_consensus
HA	7	H	 inference description
HA	8	Q	 inference description
HA	94	N	inference description
HA	121	N	inference description 
```

After cloning the mira-oxide repo, execute this command to create a mutations of interest table for the samples:

```
cargo run -p mutations_of_interest_table -- -i <PATH>/DAIS_ribosome.seq -r <PATH>/ref_table.txt -o <PATH>/outputs.csv -m <PATH>/muts_of_interest.txt
```

If you would like the output to have another deliminator (default: ","), then the `-d` flag can be used to pass another deliminator.

### The Mutations of Interest Table output should be structured like this (comma delimited):

```
sample, reference_strain,gisaid_accession,ctype,dais_reference,protein,nt_mutation,aa_mutation,phenotypic_consequence
s3_4,A/Georgia/12/2022,EPI_ISL_15724408,A_HA_H1,CALI07,HA,G:923:A,R:308:K,Inference description
s3_4,A/Wisconsin/67/2022,EPI_ISL_15928538,A_HA_H1,CALI07,HA,G:923:A,R:308:K,Inference description
s3_4,A/Michigan/383/2018,EPI_ISL_320690,A_HA_H1,CALI07,HA,G:923:A,R:308:K,Inference description
s3_4,A/Michigan/383/2018,EPI_ISL_320690,A_HA_H1,CALI07,HA,G:924:A,R:308:K,Inference description
s3_4,A/West Virginia/30/2022,EPI_ISL_15724406,A_HA_H1,CALI07,HA,G:923:A,R:308:K,Inference description
s1_2,A/duck/Bangladesh/19097/2013,EPI_ISL_151729,A_PB1,HK4801,PB1,C:295:.,H:99:.,amino acid information missing
s1_2,A/duck/Bangladesh/19097/2013,EPI_ISL_151729,A_PB1,HK4801,PB1,A:296:.,H:99:.,amino acid information missing
s1_2,A/duck/Bangladesh/19097/2013,EPI_ISL_151729,A_PB1,HK4801,PB1,C:297:.,H:99:.,amino acid information missing
```


----------------------------------------------------------------------------------
## Mutations of Interest Package Version 0.3.0 
![Alt text](../assets/images/coming_soon.png)
