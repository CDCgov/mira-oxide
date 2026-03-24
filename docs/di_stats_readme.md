# ADI Stats

The di-stats utility reads coverage data from IRMA assembly outputs and calculates DI statistics for the 5′ and 3′ ends of genomic segments. It outputs a summary table for all samples in an assembly directory that includes the run ID, sample ID, segment, and the computed DI ratios. 

## Commands
-a, --assemblies-dir <PathBuf>
    The file path to the samples folders with IRMA outputs.

-r, --run-id <PathBuf>
    The run-d associated with teh Mira run.

Outputs are made whatever the program is being run.

After cloning the mira-oxide repo, execute this command to create the table of nucleotide differences and their positions for the samples provided:

```bash
 cargo run -- di_stats -a <PATH_TO_MIRA_OUTPUTS> -r <RUNID>
```

Or run the biniary (inside or outside of container):
```bash
 mira-oxide di-stats -a <PATH_TO_MIRA_OUTPUTS> -r <RUNID>
```

## The DI stats output should be structured like this

```text
run_id	sample_id	segment	prime5	prime3	di_ratios_5prime_3prime
runid123	sample_1	B_HA	0.916	2.773	(0.916;2.773)
runid123	sample_1	B_MP	0.513	0.61	(0.513;0.61)
runid123	sample_1	B_NA	0.71	1.086	(0.71;1.086)
runid123	sample_1	B_NP	0.705	0.659	(0.705;0.659)
runid123	sample_1	B_NS	0.613	0.629	(0.613;0.629)
runid123	sample_1	B_PA	0.618	1.08	(0.618;1.08)
runid123	sample_1	B_PB1	1.141	1.288	(1.141;1.288)
runid123	sample_1	B_PB2	0.941	0.939	(0.941;0.939)
runid123	sample_2	B_HA	0.778	1.881	(0.778;1.881)
runid123	sample_2	B_MP	0.477	0.709	(0.477;0.709)
runid123	sample_2	B_NA	0.285	2.525	(0.285;2.525)
runid123	sample_2	B_NP	0.811	0.666	(0.811;0.666)
runid123	sample_2	B_NS	0.595	0.606	(0.595;0.606)
runid123	sample_2	B_PA	0.712	1.667	(0.712;1.667)
runid123	sample_2	B_PB2	0.664	0.923	(0.664;0.923)
runid123	sample_3	A_HA_H1	1.048	0.551	(1.048;0.551)
runid123	sample_3	A_MP	0.698	0.634	(0.698;0.634)
runid123	sample_3	A_NA_N1	0.519	0.628	(0.519;0.628)
runid123	sample_3	A_NP	1.005	0.611	(1.005;0.611)
runid123	sample_3	A_NS	0.71	0.593	(0.71;0.593)
runid123	sample_3	A_PA	0.579	0.695	(0.579;0.695)
runid123	sample_3	A_PB1	0.742	0.977	(0.742;0.977)
runid123	sample_3	A_PB2	1.098	0.803	(1.098;0.803)
runid123	sample_4	A_HA_H3	0.811	0.543	(0.811;0.543)
runid123	sample_4	A_MP	0.706	0.524	(0.706;0.524)
runid123	sample_4	A_NA_N2	0.621	0.414	(0.621;0.414)
runid123	sample_4	A_NP	0.76	0.649	(0.76;0.649)
runid123	sample_4	A_NS	0.878	0.72	(0.878;0.72)
runid123	sample_4	A_PA	0.612	1.106	(0.612;1.106)
runid123	sample_4	A_PB1	0.959	0.581	(0.959;0.581)
runid123	sample_4	A_PB2	0.688	0.599	(0.688;0.599)

```
