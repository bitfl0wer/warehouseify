# warehouseify

Create your own `cargo-binstall` repository with GitHub and GitHub Actions.

## Goals

- Create repository
- Generate minisign keypair
- Add WAREHOUSE_SECRET to github repository secrets
- Add WAREHOUSE_PUBLIC to github repository variables
- use warehousify binary or action with config file specifying binary crates, specifying private key and target repo
- specify target architectures in config file (default is x86-64 only)
- specify whether crates should be auditable in config file
- specify dependency versions in config file if wanted (latest is default)
- warehousify edits target crates cargo.toml with binstall info, creates binaries, outputs them on binary and uploads them when using the gh action
