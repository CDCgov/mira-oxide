# MIRA-Oxide

**General disclaimer** This repository was created for use by CDC programs to collaborate on public health related projects in support of the [CDC mission](https://www.cdc.gov/about/cdc/#cdc_about_cio_mission-our-mission).  GitHub is not hosted by the CDC, but is a third party website used by CDC and its partners to share information and collaborate on software. CDC use of GitHub does not imply an endorsement of any one particular service, product, or enterprise. 

Use of this service is limited only to non-sensitive and publicly available data. Users must not use, share, or store any kind of sensitive data like health status, provision or payment of healthcare, Personally Identifiable Information (PII) and/or Protected Health Information (PHI), etc. under ANY circumstance.

Administrators for this service reserve the right to moderate all information used, shared, or stored with this service at any time. Any user that cannot abide by this disclaimer and Code of Conduct may be subject to action, up to and including revoking access to services.

The material embodied in this software is provided to you "as-is" and without warranty of any kind, express, implied or otherwise, including without limitation, any warranty of fitness for a particular purpose. In no event shall the Centers for Disease Control and Prevention (CDC) or the United States (U.S.) government be liable to you or anyone else for any direct, special, incidental, indirect or consequential damages of any kind, or any damages whatsoever, including without limitation, loss of profit, loss of use, savings or revenue, or the claims of third parties, whether or not CDC or the U.S. government has been advised of the possibility of such loss, however caused and on any theory of liability, arising out of or in connection with the possession, use or performance of this software.

## Overview

MIRA-Oxide is a RUST workspace that is utilized by [MIRA-NF](https://github.com/CDCgov/MIRA-NF) to perform assembly and annotation of Influenza genomes, SARS-CoV-2 genomes, the SARS-CoV-2 spike-gene and RSV genomes.

## Adding New Package to the MIRA-Oxide Workspace

Before starting be sure that you have rust nightly installed and set as default. You will also need to have Cargo installed. If you need more information about how to install those, [see here](https://rust-book.cs.brown.edu/ch01-00-getting-started.html).

If you are using VScode the rust-analyzer extension is very help with formatting and debugging.

### Step 1

Clone the repository.

```
git clone https://github.com/CDCgov/mira-oxide.git
```

### Step 2

Move into the folder made by Cargo and create new branch for your package.

```
cd mira-oxide
git checkout -b add_new_package_name
```

### Step 3

Mira-oxide requires rust-nightly to run the [zoe](https://github.com/CDCgov/zoe) crate.

Install the nightly version of rust.

```
rustup toolchain install nightly
```
Using nightly for mira-oxide

```
rustup override set nightly
```

### Step 4
Create a new package using Cargo

```
cargo new new_package_name
```

A folder with a the name that you specified should have been created. Inside that folder there should be a Cargo.toml file and a src folder containing a main.rs file.

### Step 4
Create a new package using Cargo

```
cargo new new_package_name
```

A folder with a the name that you specified should have been created. Inside that folder there should be a Cargo.toml file and a src folder containing a main.rs file.

### Step 5

Start Working! 

Be sure that your package added itself to the main mira-oxide Cargo.toml. Then add any dependencies you will need to the main mira-oxide Cargo.toml.

Then you can start working on your package.

```
cd new_package_name
```

Add your dependencies to the package's Cargo.toml and start editing your src/main.rs

### Step 6

Run your program!

To be sure that your package is working within the workspace go the workspace area (path-to-repo-folder/mira-oxide) and run this command:

```
cargo run -p new_package_name -- #any inputs needed to run your package
```

### Step 7

Provide usage documentation.

Create a README.md within your package folder. Within that README provide a description of the package, it's inputs, it's outputs and how to execute the package.

For additional information on rust workspaces, [see here](https://rust-book.cs.brown.edu/ch14-03-cargo-workspaces.html).

## Current Packages
- [mutations_of_interest_table](mutations_of_interest_table/)
- [all_sample_hamming_dist](all_sample_hamming_dist/)
- [all_sample_nt_diffs](all_sample_nt_diffs/)
  
## Public Domain Standard Notice
This repository constitutes a work of the United States Government and is not
subject to domestic copyright protection under 17 USC § 105. This repository is in
the public domain within the United States, and copyright and related rights in
the work worldwide are waived through the [CC0 1.0 Universal public domain dedication](https://creativecommons.org/publicdomain/zero/1.0/).
All contributions to this repository will be released under the CC0 dedication. By
submitting a pull request you are agreeing to comply with this waiver of
copyright interest.

## License Standard Notice
The repository utilizes code licensed under the terms of the Apache Software
License and therefore is licensed under ASL v2 or later.

This source code in this repository is free: you can redistribute it and/or modify it under
the terms of the Apache Software License version 2, or (at your option) any
later version.

This source code in this repository is distributed in the hope that it will be useful, but WITHOUT ANY
WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A
PARTICULAR PURPOSE. See the Apache Software License for more details.

You should have received a copy of the Apache Software License along with this
program. If not, see http://www.apache.org/licenses/LICENSE-2.0.html

The source code forked from other open source projects will inherit its license.

## Privacy Standard Notice
This repository contains only non-sensitive, publicly available data and
information. All material and community participation is covered by the
[Disclaimer](DISCLAIMER.md)
and [Code of Conduct](code-of-conduct.md).
For more information about CDC's privacy policy, please visit [http://www.cdc.gov/other/privacy.html](https://www.cdc.gov/other/privacy.html).

## Contributing Standard Notice
Anyone is encouraged to contribute to the repository by [forking](https://help.github.com/articles/fork-a-repo)
and submitting a pull request. (If you are new to GitHub, you might start with a
[basic tutorial](https://help.github.com/articles/set-up-git).) By contributing
to this project, you grant a world-wide, royalty-free, perpetual, irrevocable,
non-exclusive, transferable license to all users under the terms of the
[Apache Software License v2](http://www.apache.org/licenses/LICENSE-2.0.html) or
later.

All comments, messages, pull requests, and other submissions received through
CDC including this GitHub page may be subject to applicable federal law, including but not limited to the Federal Records Act, and may be archived. Learn more at [http://www.cdc.gov/other/privacy.html](http://www.cdc.gov/other/privacy.html).

## Records Management Standard Notice
This repository is not a source of government records, but is a copy to increase
collaboration and collaborative potential. All government records will be
published through the [CDC web site](http://www.cdc.gov).

## Additional Standard Notices
Please refer to [CDC's Template Repository](https://github.com/CDCgov/template) for more information about [contributing to this repository](https://github.com/CDCgov/template/blob/main/CONTRIBUTING.md), [public domain notices and disclaimers](https://github.com/CDCgov/template/blob/main/DISCLAIMER.md), and [code of conduct](https://github.com/CDCgov/template/blob/main/code-of-conduct.md).
