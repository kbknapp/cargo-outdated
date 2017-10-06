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

