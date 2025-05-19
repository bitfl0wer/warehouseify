<img src="./.github/assets/logo.svg" align="left" alt="A hand-drawn pictogram of a simple warehouse. Rectangular box with wide, open garage door and a pointy roof. Inside the open garage door, one can see three boxes stacked on top of each other. Two boxes are on the bottom, one box is on top of the other two boxes. The pictogram is 50% gray." width="128px" height="auto"></img>

### `warehousify`

Create and maintain your own [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall) repository.

</br>

## Crate overview

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

## Roadmap

- [ ] Feature-gate internet connectivity: Allow for completely local building of crates, providing all listed crates are declared as locally available
- [ ] The `[dependencies]` section of the config.toml file should allow specifying a binstall repository
- [ ] Cross-compilation using [cross](https://crates.io/crates/cross)

##### Logo

“warehousify” Icon Source: https://www.iconfinder.com/icons/9165464/warehouse_storage_icon, published under [CC BY 4.0 license](https://creativecommons.org/licenses/by/4.0/). Created by [“khushmeen icons”](https://www.iconfinder.com/khushmeen-icons)
