<a name="v0.10.1"></a>
## v0.10.1 (2021-11-16)

* Fix acquiring package cache lock (`d38926b`)

<a name="v0.10.0"></a>
## v0.10.0 (2021-11-10)

* Adds the ability to run offline against a local registry via `--offline` (`2a5bf77`)

<a name="v0.9.18"></a>
## v0.9.18 (2021-11-10)

* Improve errors and unwraps (`4585424 `)
* Upgrade cargo to support the 2021 edition (`235ad47`)
* Add packaging status for cargo-outdated in README (`f39c8de `)
* Update dependencies for openssl 3.0 support (`a22e1cd `)
* Fix lints (`8a74214 `)
* Add the missing `cargo-features` entry in `Manifest`. (`a3f732e `)
* Added new CI stages for linting and clippy, ran rustfmt and clippy to fix any current issues, added .cargo/config.toml for a clippy alias (`99bc8a6 `)
* This patch fixes the feature issue where package is used to redirect a feature to a specific version of the dependency (`97a2e29 `)

<a name="0.8.0"></a>
## 0.8.0

* Update Cargo dependency for compatibility with Rust 2018 edition

<a name="0.7.1"></a>
## 0.7.1 (2018-09-23)

#### Documentation

*   update contributor list ([eae6c697](https://github.com/kbknapp/cargo-outdated/commit/eae6c697b07d0453a5fcf5bbaad397bc5c649f59))
*   updates usage text ([642ecbd1](https://github.com/kbknapp/cargo-outdated/commit/642ecbd1e1b40702142d644c560e39d8d58c3f82))

#### Bug Fixes

*   Remove extra reference taking per clippy. ([9efa1e8d](https://github.com/kbknapp/cargo-outdated/commit/9efa1e8da16461b4d3f698b4d4c7343b64c5bd92))

#### Others

*   Update dependencies and related codes for compatibility. ([d5813a13](https://github.com/kbknapp/cargo-outdated/pull/135/commits/d5813a13b538981e9ed7e60dc9f9c804ef9fe24e))

<a name="v0.7.0"></a>
## v0.7.0 (2017-11-23)

#### Bug Fixes

*   Treats optional dependencies as features as well (closes [#100](https://github.com/kbknapp/cargo-outdated/issues/100), [f67634f](https://github.com/kbknapp/cargo-outdated/pull/107/commits/f67634f1ccda77a1ff86e72d6f0e32e6c94ee717))
*   Filters yanked packages out from candidates (closes [#101](https://github.com/kbknapp/cargo-outdated/issues/101), [c42a4ef](https://github.com/kbknapp/cargo-outdated/pull/109/commits/c42a4ef33e6242c26b94017be2e6dee3136072f3))
*   Rewrites the algorithm of dependency tree comparison (closes [#105](https://github.com/kbknapp/cargo-outdated/issues/105), [5cd414a](https://github.com/kbknapp/cargo-outdated/pull/111/commits/5cd414aa766b5d7ca1bc8ec8687d8af91878116c))

#### Improvements

*   Uses [`docopt`](https://crates.io/crates/docopt) instead of [`clap`](https://crates.io/crates/clap) to make `cargo-outdated` more native (closes [#110](https://github.com/kbknapp/cargo-outdated/issues/110), [71cd8ed](https://github.com/kbknapp/cargo-outdated/pull/112/commits/71cd8edb693fef2c24df352ff8d087fd06c4ce4e), [9497bd8](https://github.com/kbknapp/cargo-outdated/pull/114/commits/9497bd8d9b365db234f2ce02b3f7cd988752931a))

#### Documentation

*   Adds crates.io version badge to readme ([7e64221](https://github.com/kbknapp/cargo-outdated/pull/104/commits/7e64221c30fbbcc5e9dcfacf12537ba3ac28207c))
*   Removes dependency graph from readme ([3792687](https://github.com/kbknapp/cargo-outdated/pull/113/commits/37926871132de526519ddbd39790d357bc5ce28a))

<a name="v0.6.3"></a>
## v0.6.3 (2017-11-07)

#### Documentation

*   Adds a CONTRIBUTORS.md and a just target to update it ([062845e](https://github.com/kbknapp/cargo-outdated/pull/102/commits/062845e6a9e54c31a9bfe42307115e80decf47b2))

#### Others

*   Adds crates.io categories and badges ([e99f31d](https://github.com/kbknapp/cargo-outdated/pull/102/commits/e99f31d39b0429c1405eff7e93b9310deeebc56d))

<a name="v0.6.2"></a>
## v0.6.2 (2017-10-28)

#### Bug Fixes

*   Replaces relative paths with absolute ones in latest manifests (closes [#96](https://github.com/kbknapp/cargo-outdated/issues/96), [ec431cd](https://github.com/kbknapp/cargo-outdated/pull/97/commits/ec431cd1bfe6680b4ccd89703c05a9840874c1ae))

<a name="v0.6.1"></a>
## v0.6.1 (2017-10-25)

#### Bug Fixes

*   Fixes `--exit-code`, `--color` (upstream) (closes [#63](https://github.com/kbknapp/cargo-outdated/issues/63), [4d4b6a8](https://github.com/kbknapp/cargo-outdated/pull/90/commits/4d4b6a85b9a6e2c212683eee35efc421188c2051))
*   Calls `Source::update()` on non-default sources before `Registry::query()` (closes [#91](https://github.com/kbknapp/cargo-outdated/issues/91), [9e7b774](https://github.com/kbknapp/cargo-outdated/pull/93/commits/9e7b774b833c38e4b9ea842b205348ab2576034d))

#### Performance

*   Replaces `format!()` with `Path.join()` (closes [#73](https://github.com/kbknapp/cargo-outdated/issues/73), [4d28c02](https://github.com/kbknapp/cargo-outdated/pull/94/commits/4d28c028fcd7cfce42df7e9223858ad5b046c9e7))

#### Tests

*   Runs Travis CI only on `master` to avoid redundant builds

<a name="v0.6.0"></a>
## v0.6.0 (2017-10-21)

#### Improvements

*   Queries [`crates.io`](https://crates.io) API for new versions with a channel-aware policy for latest ones (can be ignored by `--aggressive`) (closes [#75](https://github.com/kbknapp/cargo-outdated/issues/75), [7d57929](https://github.com/kbknapp/cargo-outdated/pull/87/commits/7d5792943fd28e17e57589520818b08b55cb667f))

#### Bug Fixes

*   Queries [`crates.io`](https://crates.io) API for feature changes to avoid "Package does not have these features" errors and warns user of obsolete features (can be suppressed by `--quiet`) (closes [#84](https://github.com/kbknapp/cargo-outdated/issues/84), [7d57929](https://github.com/kbknapp/cargo-outdated/pull/87/commits/7d5792943fd28e17e57589520818b08b55cb667f))

#### Documentation

*   Updates dependency graph in `README.md` (closes [#86](https://github.com/kbknapp/cargo-outdated/issues/86), [cf773eb](https://github.com/kbknapp/cargo-outdated/pull/88/commits/cf773eb1643ee4770dc107579f668ea2a5fd6d3a))

#### Others

*   Updates `cargo` to `0.22.0` ([29ce666](https://github.com/kbknapp/cargo-outdated/pull/85/commits/29ce6661cab50dcc9624d0a02be0facf4d5da067))

<a name="v0.5.3"></a>
## v0.5.3 (2017-10-10)

#### Improvements

*   Provides `--workspace` flag to enforce workspace mode so that it can loop through workspace members even if it's not executed against a virtual manifest (closes [#81](https://github.com/kbknapp/cargo-outdated/issues/81), [f690a7a](https://github.com/kbknapp/cargo-outdated/pull/82/commits/f690a7a22a3c1f56e67c7ee784e69d96f537c301))

<a name="v0.5.2"></a>
## v0.5.2 (2017-10-06)

#### Documentation

*   Briefly explains how `cargo-outdated` works in `README.md` ([8c35c61](https://github.com/kbknapp/cargo-outdated/commit/8c35c6148b4a29d50b55f1b064045e611fc5aa9b))

#### Features

*   Loops through all workspace members if executed against a virtual manifest (closes [#58](https://github.com/kbknapp/cargo-outdated/issues/58), [cd36aed](https://github.com/kbknapp/cargo-outdated/commit/cd36aed8f6b540d58ff4eb805cb2a20985f0122e))

#### Bug Fixes

*   Fixes missing dependency issue for debug build (closes [#77](https://github.com/kbknapp/cargo-outdated/issues/77), [c82e928](https://github.com/kbknapp/cargo-outdated/pull/78/commits/c82e92859e4659effcc08362081042b441004a1d))


#### Tests

*   Debug build is now part of CI ([05ada44](https://github.com/kbknapp/cargo-outdated/pull/78/commits/05ada447863f775ff58e6bfcaa764582af62f2cc))

<a name="v0.5.1"></a>
## v0.5.1 (2017-09-23)


#### Documentation

*   Fixes a typo ([38e37c6](https://github.com/kbknapp/cargo-outdated/pull/66/commits/38e37c6ee77a6ff252bb0702033d7a0b03eac226))

#### Improvements

*   Enables `--all-features` by default (closes [#57](https://github.com/kbknapp/cargo-outdated/issues/57), [f24c3a6](https://github.com/kbknapp/cargo-outdated/pull/64/commits/f24c3a6a8e050cbb651661bfbc9221546d987c41))
*   Prints a dashed line under the table header ([b076bb1](https://github.com/kbknapp/cargo-outdated/pull/65/commits/b076bb144818b2c5d7efcc3af0acf85ae83f44e1))

#### Bug Fixes

*   Correctly shows error messages (closes [#60](https://github.com/kbknapp/cargo-outdated/issues/60), [daab865](https://github.com/kbknapp/cargo-outdated/pull/61/commits/daab865647715cf467fc28f1333afcd1fe2cf447))
*   Excludes default features if not explicitly specified by user (closes [#69](https://github.com/kbknapp/cargo-outdated/issues/69), [7074fc8](https://github.com/kbknapp/cargo-outdated/pull/70/commits/7074fc8754d0cf231ff84070307ee92c1cedf065))

<a name="v0.5.0"></a>
## v0.5.0 (2017-09-18)


#### Refactoring

*   Introduces [`cargo`](https://crates.io/crates/cargo) as a dependency ([0539a61](https://github.com/kbknapp/cargo-outdated/pull/51/commits/0539a619d30175fd287a979a9eecb1143df0f2f6))

#### Improvements

*   Replaces `RM` with `Removed` (closes [#46](https://github.com/kbknapp/cargo-outdated/issues/46))
*   Adds `Kind`, `Platform` in output

#### Features

*   Supports `cargo` workspaces (closes [#28](https://github.com/kbknapp/cargo-outdated/issues/28))
*   Supports embedded dependencies (fixes [#50](https://github.com/kbknapp/cargo-outdated/issues/50))
*   Supports build/development/target-specific dependencies (closes [#20](https://github.com/kbknapp/cargo-outdated/issues/20), fixes [#49](https://github.com/kbknapp/cargo-outdated/issues/49))
*   Adds `--all-features`, `--features`, `--no-default-features`



<a name="v0.4.0"></a>
## v0.4.0 (2017-08-04)


#### Documentation

*   Spelling ([6d309060](https://github.com/kbknapp/cargo-outdated/commit/6d3090601d03694838a848e044f157764d0271cb))

#### Bug Fixes

*   Sets bin.path in the temp manifest ([a0231de5](https://github.com/kbknapp/cargo-outdated/commit/a0231de51428e5238dcab0d73cdce2d2443f8a7e), closes [#41](https://github.com/kbknapp/cargo-outdated/issues/41))
*   Correctly handles dependencies with multiple occurrences ([03d3e74cf](https://github.com/kbknapp/cargo-outdated/commit/03d3e74cf38156adecc1620271ec8beb9c442865))



<a name="v0.3.0"></a>
## v0.3.0 (2016-12-05)


#### Features

*   adds a --manifest-path and --lockfile-path to allow use with other projects ([5f886d27](https://github.com/kbknapp/cargo-outdated/commit/5f886d27d3fefbc0b7fec9ffef651c137f58420d), closes [#29](https://github.com/kbknapp/cargo-outdated/issues/29))

<a name="v0.2.0"></a>
## v0.2.0

* **Exit Codes:**  adds feature for custom exit code on new vers ([61c8bb9b](https://github.com/kbknapp/cargo-outdated/commit/61c8bb9b52af8745fd16fad646bc2f4dcce336c7), closes [#23](https://github.com/kbknapp/cargo-outdated/issues/23))

#### Improvements

*   sort output ([b137e050](https://github.com/kbknapp/cargo-outdated/commit/b137e050ffb861f7ff725324be5cdb527d724a49))


<a name="v0.1.3"></a>
## v0.1.3 (2015-11-14)


#### Documentation

*   adds demo ([c2192aac](https://github.com/kbknapp/cargo-outdated/commit/c2192aac903e764a43fc103251e56ce50b89a8eb))
*   updates readme with cargo install instructions ([e936a454](https://github.com/kbknapp/cargo-outdated/commit/e936a45443fc02ab65be15d6a872609a95f7dc00))

#### Bug Fixes

*   fixes build error on windows due to upstream dep ([af4e1a70](https://github.com/kbknapp/cargo-outdated/commit/af4e1a704a70d5524e76c9ad6fd320cd576c4a2c))

<a name="v0.1.1"></a>
### v0.1.1 (2015-11-04)


#### Documentation

*   adds crate level docs ([8ba28c73](https://github.com/kbknapp/cargo-outdated/commit/8ba28c73e084bf0535e0df72653c529886d025a5))

#### Improvements

*   various fixes from clippy run ([b8b633fc](https://github.com/kbknapp/cargo-outdated/commit/b8b633fc148b8be38fec8a8efc73d30bc2917716))



<a name="v0.1.0"></a>
## v0.1.0 Initial Implementation (2015-08-11)

### Features
* Initial implementation ([e5d5a82e](https://github.com/kbknapp/cargo-outdated/commit/e5d5a82e95b86f088c53fe5665dc4f8219b7db49))

#### Improvements

*   adds better error handling ([9032454c](https://github.com/kbknapp/cargo-outdated/commit/9032454cd1fcbd2d1cadbb924b8664ced04e2406))

#### Documentation

* **CHANGELOG.md:**  adds a changelog ([9d1c1601](https://github.com/kbknapp/cargo-outdated/commit/9d1c1601c0729a6f60d51c86936a061f1376b06a))
* **README.md:**  adds a readme ([67bc5556](https://github.com/kbknapp/cargo-outdated/commit/67bc555669159f11907f9bb90913e45af232b277))

