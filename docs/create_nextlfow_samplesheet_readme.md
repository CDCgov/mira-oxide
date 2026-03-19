# Create Nextflow Samplesheet

This Rust code defines a CLI tool that generates a Nextflow-compatible samplesheet CSV from an input samplesheet and a sequencing run directory. It supports both ONT (Oxford Nanopore) and non-ONT (e.g. Illumina paired-end) experiments.

## Illumina Samplesheet Input
```
Sample ID,Sample Type,Unnamed: 2
sample_1,Test,nan
sample_2,Test,nan
sample_3,Test,nan
sample_4,Test,nan
```

## ONT Samplesheet Input
```
Barcode #,Sample ID,Sample Type
barcode27,s1,Test
barcode37,s2,Test
barcode41,s3,Test
```

## Run Directory Path to FASTQ files
The path to the run folder containing your fastq files. 

For Illumina that is to the fastq folder itself:
```
test_020426/flu_wgs_illumina/fastqs/
```

For ONT that will be to the runfolder that the fastq_pass (with the cat_fastq inside) is copied to in the MIRA-NF pipeline
```
test_020426/flu_wgs_illumina/
```
**The script is sppecifically lookinf for {runpath}/fastq_pass/cat_fastqs/{id}_nf_combined.fastq* in ONT's case**

## MIRA-NF Experiment Type Options:
```
Flu-ONT
SC2-Spike-Only-ONT
Flu-Illumina
SC2-Whole-Genome-ONT
SC2-Whole-Genome-Illumina
RSV-Illumina
RSV-ONT
```

## Ilumina Output
```
sample,fastq_1,fastq_2,sample_type
sample_1,test_020426/flu_wgs_illumina/fastqs/sample_1_R1.fastq.gz,test_020426/flu_wgs_illumina/fastqs/sample_1_R2.fastq.gz,Test
sample_2,test_020426/flu_wgs_illumina/fastqs/sample_2_R1.fastq.gz,test_020426/flu_wgs_illumina/fastqs/sample_2_R2.fastq.gz,Test
sample_3,test_020426/flu_wgs_illumina/fastqs/sample_3_R1.fastq.gz,test_020426/flu_wgs_illumina/fastqs/sample_3_R2.fastq.gz,Test
sample_4,test_020426/flu_wgs_illumina/fastqs/sample_4_R1.fastq.gz,test_020426/flu_wgs_illumina/fastqs/sample_4_R2.fastq.gz,Test
```
## ONT Output
```
sample,barcodes,fastq_1,fastq_2,sample_type
s1,barcode27,test_020426/flu_wgs_ont/outputs/fastq_pass/cat_fastqs/s1_nf_combined.fastq.gz,,Test
s2,barcode37,test_020426/flu_wgs_ont/outputs/fastq_pass/cat_fastqs/s2_nf_combined.fastq.gz,,Test
s3,barcode41,test_020426/flu_wgs_ont/outputs/fastq_pass/cat_fastqs/s3_nf_combined.fastq.gz,,Test
```

After cloning the mira-oxide repo, execute this command to create a hamming distance matrix for the samples provided:

```bash
cargo run -- create-nextflow-samplesheet -s <RUNDIR>/samplesheet.csv -r <RUNDIR> -e <experiment_type>
```