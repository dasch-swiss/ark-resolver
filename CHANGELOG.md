# Changelog

## [1.14.1](https://github.com/dasch-swiss/ark-resolver/compare/v1.14.0...v1.14.1) (2026-02-26)


### Bug Fixes

* **ci:** Fix PROD deployment by matching publish job condition to release event trigger ([c5c3e6a](https://github.com/dasch-swiss/ark-resolver/commit/c5c3e6aaeee607143dd923b2944bd4220036eeff))

## [1.14.0](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.13.0...ark_resolver-v1.14.0) (2026-02-25)


### Maintenances

* Add .claude/worktrees/ to .gitignore ([2393c76](https://github.com/dasch-swiss/ark-resolver/commit/2393c76bde044fb00a9522ba45239c53d8dba201))
* **deploy:** Add auto-deployment to prod (INFRA-1022) ([5c224aa](https://github.com/dasch-swiss/ark-resolver/commit/5c224aa295c0a9d366c9871dd7be127c8883a2bf))
* Extract duplicated _report_error_to_sentry into error_diagnostics ([6fb71ce](https://github.com/dasch-swiss/ark-resolver/commit/6fb71ce3bdc416cc72a640212449ef9f862af033))
* Replace fragile string matching with ArkUrlException subclasses ([5f271d8](https://github.com/dasch-swiss/ark-resolver/commit/5f271d8816697af8add59ed800eb07524b042055))


### Documentation

* Add BR comments for UUID_TOO_LONG threshold, Sentry skip, and check ordering ([fbc2b34](https://github.com/dasch-swiss/ark-resolver/commit/fbc2b340df77bb4d3ec0a22859a321cc52c75d47))


### Enhancements

* Integrate Docker Scout CVE scanning, SBOM, and dependency audits ([2d9d071](https://github.com/dasch-swiss/ark-resolver/commit/2d9d071088186a8b73831666aebafb27575ed61a))
* Structured JSON error responses with diagnostic codes and hints ([8e4c917](https://github.com/dasch-swiss/ark-resolver/commit/8e4c91754db48051c9a022a8d66186fc0d83bbbb))


### Bug Fixes

* Address PR review — least-privilege permissions and conditional login ([a97e7ee](https://github.com/dasch-swiss/ark-resolver/commit/a97e7ee428fda8ec71ca0605d991d5952d97193e))
* Handle numeric HTML entities in _clean_html_entities ([009d8e4](https://github.com/dasch-swiss/ark-resolver/commit/009d8e43969dd9cebda1a1103479b899354a5533))
* Make Docker Scout compare best-effort for missing baseline ([a0506b1](https://github.com/dasch-swiss/ark-resolver/commit/a0506b1110aa6f3d4ca45968e7495c8dfa27ecd2))
* Remove continue-on-error from Scout compare step ([d05e4b3](https://github.com/dasch-swiss/ark-resolver/commit/d05e4b32771f56d316ea2f5b337f59bd01eb6e72))
* Resolve CI failures in security workflow ([d1e9b9b](https://github.com/dasch-swiss/ark-resolver/commit/d1e9b9b94098c4e206b775c5974f68611872de49))
* Revert builder Python to 3.12 to match Alpine 3.23 system Python ([9112c2f](https://github.com/dasch-swiss/ark-resolver/commit/9112c2fcb8d4d1a453d445eb0bb55b32495fde35))
* Upgrade pyo3 to 0.24 and bump Alpine base to 3.23 ([aae7638](https://github.com/dasch-swiss/ark-resolver/commit/aae76389bb17944100a91813b955aaa3f227f105))
* Upgrade vulnerable Rust dependencies and ignore pyo3 advisory ([11f0ddf](https://github.com/dasch-swiss/ark-resolver/commit/11f0ddf199c65ec0561709af796414ab1b1b5b32))
* Upgrade wheel to &gt;=0.46.2 to fix CVE-2026-24049 ([906a7bc](https://github.com/dasch-swiss/ark-resolver/commit/906a7bceecfbbdda6ffb7c4a6aa77c97919d63c7))
* Use clean string format for INVALID_UUID_CHARS detail field ([f136ba3](https://github.com/dasch-swiss/ark-resolver/commit/f136ba3716d80b1bbc96353c220403605e596b05))
* Use python:3.12-alpine3.23 runtime to match builder ABI ([e7b7057](https://github.com/dasch-swiss/ark-resolver/commit/e7b70579f03f2b72233bd0308fb27e37d7ef1dc8))

## [1.13.0](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.12.2...ark_resolver-v1.13.0) (2026-02-15)


### Maintenances

* Add early debug logging ([#137](https://github.com/dasch-swiss/ark-resolver/issues/137)) ([6c5cf21](https://github.com/dasch-swiss/ark-resolver/commit/6c5cf2145de4eba2de21d6fb9b0258c0e0daad05))
* Add environment variable logging ([#135](https://github.com/dasch-swiss/ark-resolver/issues/135)) ([048ce9d](https://github.com/dasch-swiss/ark-resolver/commit/048ce9dac6e01cecd0a26375ea3da8830382a65e))
* Rename sentry debug env variable ([#136](https://github.com/dasch-swiss/ark-resolver/issues/136)) ([4505a92](https://github.com/dasch-swiss/ark-resolver/commit/4505a925c12f57ef27d9758536c0a31be884dbbc))
* Use cached Rust settings in convert route ([1867a38](https://github.com/dasch-swiss/ark-resolver/commit/1867a38b8af4e6d6e768d0b3e586ff50fb63839d))


### Documentation

* **learnings:** Add PyO3 shadow execution parity learnings ([033b6e1](https://github.com/dasch-swiss/ark-resolver/commit/033b6e1bf5bf9fead674a5500bdcdfe49e31c397))
* Make examples inline with domain ([#128](https://github.com/dasch-swiss/ark-resolver/issues/128)) ([375dee3](https://github.com/dasch-swiss/ark-resolver/commit/375dee315f7fa4dcad60848a831816c25469ff52))
* Note redirect route parallel execution in CLAUDE.md ([5dd64ee](https://github.com/dasch-swiss/ark-resolver/commit/5dd64eee4aaece9d6a4a653e9a1445fe3b1680fc))


### Enhancements

* Add custom Sentry fingerprinting to group errors by category ([1a5ebdc](https://github.com/dasch-swiss/ark-resolver/commit/1a5ebdcf356587108241e592a11751ca2e353876))
* Add parallel shadow execution to redirect route ([72db278](https://github.com/dasch-swiss/ark-resolver/commit/72db278be50216259de4403f916559699ee6a91d))
* Cache Rust settings at startup for parallel validation ([5840733](https://github.com/dasch-swiss/ark-resolver/commit/58407330aa1e94b622860ef91006d8ad64822ec2))


### Bug Fixes

* Add http config fetching in Rust ([#127](https://github.com/dasch-swiss/ark-resolver/issues/127)) ([281f5bf](https://github.com/dasch-swiss/ark-resolver/commit/281f5bf26b5abf1a838ba48369f2ec8acc95847b))
* Add http config fetching in Rust (3) ([#134](https://github.com/dasch-swiss/ark-resolver/issues/134)) ([b931895](https://github.com/dasch-swiss/ark-resolver/commit/b931895591e3b9d7260ab59959721e0b44a17ece))
* Early debug logging ([#138](https://github.com/dasch-swiss/ark-resolver/issues/138)) ([8476bc3](https://github.com/dasch-swiss/ark-resolver/commit/8476bc39b6ff13cf52c850c15c58eacd96bf3fc3))
* Http config fetching in Rust ([#129](https://github.com/dasch-swiss/ark-resolver/issues/129)) ([f45ece4](https://github.com/dasch-swiss/ark-resolver/commit/f45ece40144193e5da73796df6ead49fac5a78ac))
* Http config fetching in Rust ([#130](https://github.com/dasch-swiss/ark-resolver/issues/130)) ([6c4a35a](https://github.com/dasch-swiss/ark-resolver/commit/6c4a35a428638e3af96392f96ea83b9c6b11a1ef))
* Http config fetching in Rust ([#131](https://github.com/dasch-swiss/ark-resolver/issues/131)) ([17f6e6e](https://github.com/dasch-swiss/ark-resolver/commit/17f6e6e640f5e9136b0d36a4c211ec7daf4c3875))
* Http config fetching in Rust ([#132](https://github.com/dasch-swiss/ark-resolver/issues/132)) ([e276b86](https://github.com/dasch-swiss/ark-resolver/commit/e276b869baf24c49ddfa78af846bafd79c22f42c))
* Http config fetching in Rust ([#133](https://github.com/dasch-swiss/ark-resolver/issues/133)) ([dd35ed3](https://github.com/dasch-swiss/ark-resolver/commit/dd35ed333e859f3f5e57981ab3cb82356f4e759c))
* Re-raise KeyboardInterrupt and SystemExit in BaseException catch ([d7c3dad](https://github.com/dasch-swiss/ark-resolver/commit/d7c3dad0553bfb3e3de39aeea8bc23a6a2478106))
* Rust tracing initialization ([#139](https://github.com/dasch-swiss/ark-resolver/issues/139)) ([f92db0f](https://github.com/dasch-swiss/ark-resolver/commit/f92db0f0ad5a89748d8beb1ab5653a9b4847db04))
* Simplify config ([#125](https://github.com/dasch-swiss/ark-resolver/issues/125)) ([fa1d1a2](https://github.com/dasch-swiss/ark-resolver/commit/fa1d1a2759a1de07d9e86af0512cddf3ea85ff56))
* Uppercase v1 project IDs in Rust to match Python parity ([c7e1b76](https://github.com/dasch-swiss/ark-resolver/commit/c7e1b76693cc09b7d5e92ad505ac3dfb1a93688b))
* Use correct environment variable ([#126](https://github.com/dasch-swiss/ark-resolver/issues/126)) ([089de08](https://github.com/dasch-swiss/ark-resolver/commit/089de08dd7098bf0a815dd699591be0a3c80c796))
* Use correct environment variable for registry file ([#123](https://github.com/dasch-swiss/ark-resolver/issues/123)) ([1d9906c](https://github.com/dasch-swiss/ark-resolver/commit/1d9906c9bd42839f8446e0df035878e4486bb1cd))


### Tests

* Add Python-Rust redirect URL parity tests ([aec349c](https://github.com/dasch-swiss/ark-resolver/commit/aec349c6c85e5cf7ee2b182d4199752236b99d06))
* Add redirect parity cases to smoke test ([28e6005](https://github.com/dasch-swiss/ark-resolver/commit/28e6005bed290417885d851bb389fd57f01443f1))


### Styles

* Fix formatting and lint compliance ([6796173](https://github.com/dasch-swiss/ark-resolver/commit/6796173b853945a4cb8241baf4c5ebd4cb5b570c))
* Remove unused noqa directive on BaseException catch ([33534c5](https://github.com/dasch-swiss/ark-resolver/commit/33534c55d407ba4730fd2a986e4c68ccfb9325df))

## [1.12.2](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.12.1...ark_resolver-v1.12.2) (2025-07-24)


### Maintenances

* Add caching and smoke-test execution to CI ([#118](https://github.com/dasch-swiss/ark-resolver/issues/118)) ([cace974](https://github.com/dasch-swiss/ark-resolver/commit/cace974fa009b45f1b784dcde03e1b41307188be))
* Add cargo-nextest and update Rust toolchain ([#116](https://github.com/dasch-swiss/ark-resolver/issues/116)) ([fe709a5](https://github.com/dasch-swiss/ark-resolver/commit/fe709a5839cd56b772664ba95db2738287f477a0))
* Add claude and typechecking ([#100](https://github.com/dasch-swiss/ark-resolver/issues/100)) ([5162b2b](https://github.com/dasch-swiss/ark-resolver/commit/5162b2bf14778497686026d267cb049a56339d6a))
* Add parallel execution framework for Python/Rust shadow validation ([#109](https://github.com/dasch-swiss/ark-resolver/issues/109)) ([bdef6ef](https://github.com/dasch-swiss/ark-resolver/commit/bdef6ef7355794209f5b252382857095e4c27e91))
* ARK URL Formatter migrated to hexagonal architecture ([#105](https://github.com/dasch-swiss/ark-resolver/issues/105)) ([f3fd637](https://github.com/dasch-swiss/ark-resolver/commit/f3fd6371dfb2afbce6c2d190c5d4580bbdf123b8))
* ARK URL Info Processing migrated to hexagonal architecture ([#107](https://github.com/dasch-swiss/ark-resolver/issues/107)) ([ebb83ce](https://github.com/dasch-swiss/ark-resolver/commit/ebb83cea57ee3389c72786876e2245cf396b71df))
* Check digit module migrated to hexagonal architecture ([#104](https://github.com/dasch-swiss/ark-resolver/issues/104)) ([3735f96](https://github.com/dasch-swiss/ark-resolver/commit/3735f964db19d83fb1583c6b5b793c77ecd58905))
* Complete ARK URL Info Processing migration to Rust ([#108](https://github.com/dasch-swiss/ark-resolver/issues/108)) ([561e0e0](https://github.com/dasch-swiss/ark-resolver/commit/561e0e0aa3eb251fcd91c1c8eb5071c5d4b74ae5))
* Complete migration of `check_digit.py` to Rust with full test parity ([#101](https://github.com/dasch-swiss/ark-resolver/issues/101)) ([8b419aa](https://github.com/dasch-swiss/ark-resolver/commit/8b419aa2723859b8323823b2824a465b6accc462))
* Extract catch_all route to dedicated redirect module ([#110](https://github.com/dasch-swiss/ark-resolver/issues/110)) ([8c07fc1](https://github.com/dasch-swiss/ark-resolver/commit/8c07fc167812f5989bf98845330d5a90a4f13c33))
* Fix cargo check and formatting errors ([#121](https://github.com/dasch-swiss/ark-resolver/issues/121)) ([c93e7f4](https://github.com/dasch-swiss/ark-resolver/commit/c93e7f472d22c39dcf7ed7b2d24b9ad66f20b22d))
* Ignore temporary Graphite branches in CI ([#119](https://github.com/dasch-swiss/ark-resolver/issues/119)) ([9cd01b9](https://github.com/dasch-swiss/ark-resolver/commit/9cd01b95acd94fe03689c5f5cc76f895b466c051))
* Implement shadow execution and logging for convert route ([#117](https://github.com/dasch-swiss/ark-resolver/issues/117)) ([fc2b081](https://github.com/dasch-swiss/ark-resolver/commit/fc2b0819309aec4831d4d7881a202a21107aeccc))
* Improve CI caching to prevent Docker build timeouts ([#120](https://github.com/dasch-swiss/ark-resolver/issues/120)) ([22fccdb](https://github.com/dasch-swiss/ark-resolver/commit/22fccdb9f595b69c7a7e7afd5c257a0611714f5c))
* Improve github CI check workflow ([#122](https://github.com/dasch-swiss/ark-resolver/issues/122)) ([db35e88](https://github.com/dasch-swiss/ark-resolver/commit/db35e885936e71959ba23b48f9f11f03d38e9201))
* Migrate ark_url_settings to hexagonal architecture ([#112](https://github.com/dasch-swiss/ark-resolver/issues/112)) ([100b246](https://github.com/dasch-swiss/ark-resolver/commit/100b24609e9f9a8d991f1c146a438f3b996772df))
* Move parsing module to domain layer ([#114](https://github.com/dasch-swiss/ark-resolver/issues/114)) ([6f747b0](https://github.com/dasch-swiss/ark-resolver/commit/6f747b044c62fa4edf495bd07f5e82060792574e))
* Project setup ([19e0dfa](https://github.com/dasch-swiss/ark-resolver/commit/19e0dfa59c586122bd36d231a84534c7e94b64eb))
* Remove old implementation ([#113](https://github.com/dasch-swiss/ark-resolver/issues/113)) ([bd52d03](https://github.com/dasch-swiss/ark-resolver/commit/bd52d031175c6bdc80c8d5ecb75566002e8d7c38))
* UUID processing migrated to hexagonal architecture and legacy code removed ([#106](https://github.com/dasch-swiss/ark-resolver/issues/106)) ([90475e2](https://github.com/dasch-swiss/ark-resolver/commit/90475e26679f3ac6c43a4d9d6f492d23b5fd2487))
* UUID Processing Migration ([#102](https://github.com/dasch-swiss/ark-resolver/issues/102)) ([7d7d72e](https://github.com/dasch-swiss/ark-resolver/commit/7d7d72e6d0b3800c4093f17753186d435eb6ef62))


### Documentation

* Add ADR and update todos ([#103](https://github.com/dasch-swiss/ark-resolver/issues/103)) ([386c2bc](https://github.com/dasch-swiss/ark-resolver/commit/386c2bcb4baf7b5a1561b17d1724864303ba620d))
* Structured logging design ([#99](https://github.com/dasch-swiss/ark-resolver/issues/99)) ([8627ad3](https://github.com/dasch-swiss/ark-resolver/commit/8627ad38ebadd95dd4ac08f9b90ace9dcf02f4e4))


### Bug Fixes

* Update justfile test target to run all Rust unit tests without PyO3 issues ([#115](https://github.com/dasch-swiss/ark-resolver/issues/115)) ([8fd6547](https://github.com/dasch-swiss/ark-resolver/commit/8fd654783cdf026934da8de0d78d5c1e0fc78e1f))


### Tests

* Extend smoke test to cover convert and redirect routes ([#111](https://github.com/dasch-swiss/ark-resolver/issues/111)) ([862eeb7](https://github.com/dasch-swiss/ark-resolver/commit/862eeb72733e83133c8b7ed5d3f629ed300c8d29))

## [1.12.1](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.12.0...ark_resolver-v1.12.1) (2025-05-05)


### Maintenances

* Add docker healthcheck (INFRA-787) ([#94](https://github.com/dasch-swiss/ark-resolver/issues/94)) ([aa51e99](https://github.com/dasch-swiss/ark-resolver/commit/aa51e9975f7e9cce295ac0d6e64da4c039bd879f))

## [1.12.0](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.11.0...ark_resolver-v1.12.0) (2025-04-03)


### Enhancements

* Add convert route ([#88](https://github.com/dasch-swiss/ark-resolver/issues/88)) ([9b379e8](https://github.com/dasch-swiss/ark-resolver/commit/9b379e87a7226f7a32affd928b49990f22c0f994))

## [1.11.0](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.10.0...ark_resolver-v1.11.0) (2025-03-31)


### Maintenances

* Add ruff formatting and linting ([#82](https://github.com/dasch-swiss/ark-resolver/issues/82)) ([2f7ef20](https://github.com/dasch-swiss/ark-resolver/commit/2f7ef20193a74b7c63b57528f702e0598993ab3c))
* Add type checking ([#83](https://github.com/dasch-swiss/ark-resolver/issues/83)) ([94613ca](https://github.com/dasch-swiss/ark-resolver/commit/94613cad086244b240b332bd0ce839de5e290f6e))
* Improve repo setup ([#80](https://github.com/dasch-swiss/ark-resolver/issues/80)) ([7b4d095](https://github.com/dasch-swiss/ark-resolver/commit/7b4d095fb638224bc766bfb08ae60b6fb8359b12))


### Enhancements

* Decode urlencoded ARK-IDs ([#85](https://github.com/dasch-swiss/ark-resolver/issues/85)) ([9162dd4](https://github.com/dasch-swiss/ark-resolver/commit/9162dd4927a8ebcaf281aca16e969eb1b023f93e))


### Bug Fixes

* Ignore case for project shortcode ([#87](https://github.com/dasch-swiss/ark-resolver/issues/87)) ([af36c51](https://github.com/dasch-swiss/ark-resolver/commit/af36c5107e88db2a7f2239875e389532e0216a19))

## [1.10.0](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.9.0...ark_resolver-v1.10.0) (2025-03-19)


### Maintenances

* Add sending opentelemetry spans to sentry ([#74](https://github.com/dasch-swiss/ark-resolver/issues/74)) ([6a2fe2c](https://github.com/dasch-swiss/ark-resolver/commit/6a2fe2cddf24b2b529671981727c38bea7ef62e4))
* Auto deploy stage ark-resolver (INFRA-769) ([#71](https://github.com/dasch-swiss/ark-resolver/issues/71)) ([66be814](https://github.com/dasch-swiss/ark-resolver/commit/66be814a3d562d64daddb8f035cf40b5519be1de))
* **ci:** Remove PR title check ([#73](https://github.com/dasch-swiss/ark-resolver/issues/73)) ([0dfe509](https://github.com/dasch-swiss/ark-resolver/commit/0dfe509c13504629339d8540a772f1e22f9f547f))


### Enhancements

* Filter out spam ([#79](https://github.com/dasch-swiss/ark-resolver/issues/79)) ([c7131e2](https://github.com/dasch-swiss/ark-resolver/commit/c7131e22eba9f5d1946159e117482b9e3a6ae4f8))

## [1.9.0](https://github.com/dasch-swiss/ark-resolver/compare/ark_resolver-v1.8.0...ark_resolver-v1.9.0) (2025-02-27)


### Maintenances

* Add a release-please action ([#43](https://github.com/dasch-swiss/ark-resolver/issues/43)) ([f0ce321](https://github.com/dasch-swiss/ark-resolver/commit/f0ce321ef768cede0589539aa7e27ded289209fd))
* Add dependabot config ([#66](https://github.com/dasch-swiss/ark-resolver/issues/66)) ([82cbc79](https://github.com/dasch-swiss/ark-resolver/commit/82cbc793c067c46abc3c7f3e660a80de107f6ada))
* Add help target ([1e3a383](https://github.com/dasch-swiss/ark-resolver/commit/1e3a3834f9da503afdd2d2ae4a3eebcc063bc69d))
* Add parallel rust config implementation ([#64](https://github.com/dasch-swiss/ark-resolver/issues/64)) ([856a058](https://github.com/dasch-swiss/ark-resolver/commit/856a0584b0ba5d9c546d37cb4c2fb4f0e1007564))
* Add release-please ([#65](https://github.com/dasch-swiss/ark-resolver/issues/65)) ([ae52f90](https://github.com/dasch-swiss/ark-resolver/commit/ae52f90f9cf422105a8c87a51502ed7d2e38bf29))
* Bump base image alpine version ([#62](https://github.com/dasch-swiss/ark-resolver/issues/62)) ([e3a20e6](https://github.com/dasch-swiss/ark-resolver/commit/e3a20e6a0051811975817b05bf724bbcd9eeb11d))
* Bump sanic related modules versions (DEV-191) ([2a55260](https://github.com/dasch-swiss/ark-resolver/commit/2a5526092ce2a6dfa2bc834db0c0fa002ffe64d7))
* Bump sanic related modules versions (DEV-191) ([2a55260](https://github.com/dasch-swiss/ark-resolver/commit/2a5526092ce2a6dfa2bc834db0c0fa002ffe64d7))
* **deps:** Bump certifi from 2021.5.30 to 2022.12.7 ([7576a93](https://github.com/dasch-swiss/ark-resolver/commit/7576a939f462c96cb6cc0ab4de9d881fc4c8ccf0))
* **deps:** Bump certifi from 2022.12.7 to 2023.7.22 ([163210d](https://github.com/dasch-swiss/ark-resolver/commit/163210d342e046e48d8e49542fec3a416ad2b500))
* **deps:** Bump certifi from 2022.12.7 to 2023.7.22 ([c2f66f6](https://github.com/dasch-swiss/ark-resolver/commit/c2f66f6505b4ecb64ea19f46ba79cb0a578f3645))
* **deps:** Bump certifi from 2023.7.22 to 2024.7.4 ([#63](https://github.com/dasch-swiss/ark-resolver/issues/63)) ([6f78595](https://github.com/dasch-swiss/ark-resolver/commit/6f78595bd724bd4d101f560e7e50d82ba417ac13))
* **deps:** Bump dependencies ([#54](https://github.com/dasch-swiss/ark-resolver/issues/54)) ([baeafef](https://github.com/dasch-swiss/ark-resolver/commit/baeafefd811f5a464a19f6144caac8858e2bb1eb))
* **deps:** Bump idna from 3.4 to 3.7 ([#57](https://github.com/dasch-swiss/ark-resolver/issues/57)) ([2141b4f](https://github.com/dasch-swiss/ark-resolver/commit/2141b4f4c7b6918023d381623bc75df8c0213e24))
* **deps:** Bump requests from 2.20.0 to 2.22.0 ([#15](https://github.com/dasch-swiss/ark-resolver/issues/15)) ([0b4e5d1](https://github.com/dasch-swiss/ark-resolver/commit/0b4e5d1ed86f94862edf68ea606262267d2497c9))
* **deps:** Bump requests from 2.25.1 to 2.31.0 ([de6ce65](https://github.com/dasch-swiss/ark-resolver/commit/de6ce651597f45a117a202c066cedb57354dc83f))
* **deps:** Bump requests from 2.25.1 to 2.31.0 ([b22d053](https://github.com/dasch-swiss/ark-resolver/commit/b22d053a583811a718fcf16f12e3a37aa64932b8))
* **deps:** Bump requests from 2.31.0 to 2.32.0 ([#60](https://github.com/dasch-swiss/ark-resolver/issues/60)) ([91e842e](https://github.com/dasch-swiss/ark-resolver/commit/91e842e467257bda0b01c9632ba9f5c56fd4bc44))
* **deps:** Bump sanic from 18.12.0 to 19.6.3 ([#16](https://github.com/dasch-swiss/ark-resolver/issues/16)) ([2f9c885](https://github.com/dasch-swiss/ark-resolver/commit/2f9c885d7f4088cfcddd9c5a8d546021646def0e))
* **deps:** Bump sanic from 19.6.3 to 19.9.0 ([#17](https://github.com/dasch-swiss/ark-resolver/issues/17)) ([02a7481](https://github.com/dasch-swiss/ark-resolver/commit/02a7481bc3000366f49ca2bff3034eef1d11e3e0))
* **deps:** Bump the backend-dependencies group with 3 updates ([#69](https://github.com/dasch-swiss/ark-resolver/issues/69)) ([d0c5b51](https://github.com/dasch-swiss/ark-resolver/commit/d0c5b51ca6930f6ac9b5e329f449d7af843a4822))
* **deps:** Bump ujson from 4.2.0 to 5.4.0 ([17b76f8](https://github.com/dasch-swiss/ark-resolver/commit/17b76f8c1367abdd3f9e07e4504ca05abc4a4236))
* **deps:** Bump urllib3 from 2.0.6 to 2.0.7 ([#56](https://github.com/dasch-swiss/ark-resolver/issues/56)) ([16087a1](https://github.com/dasch-swiss/ark-resolver/commit/16087a1f4c13ea7c85743ccfd03abaf5a691f916))
* **deps:** Bump urllib3 from 2.0.7 to 2.2.2 ([#61](https://github.com/dasch-swiss/ark-resolver/issues/61)) ([8f21269](https://github.com/dasch-swiss/ark-resolver/commit/8f21269cdc69e0cd44ff24eb848f6f79cd0a4049))
* Fix makefile ([2aeb3eb](https://github.com/dasch-swiss/ark-resolver/commit/2aeb3eb1bc98dfa782d9004130fd6d40efe98ce5))
* Fix publishing to dockerhub job: trigger when a release was published ([#50](https://github.com/dasch-swiss/ark-resolver/issues/50)) ([4808540](https://github.com/dasch-swiss/ark-resolver/commit/48085407bcbcbfad93105c73455acda173ee3b4d))
* Fixing release-please ([ab33d9a](https://github.com/dasch-swiss/ark-resolver/commit/ab33d9a2051bfaf814b7699bd07109863e7f7d9f))
* Move test and publish to Github-CI ([61d3792](https://github.com/dasch-swiss/ark-resolver/commit/61d3792330f536e856140ba36e684f3ae889ec43))
* Reformat ([09c6cf6](https://github.com/dasch-swiss/ark-resolver/commit/09c6cf63b9edd4a01296a068d70df3d6cf65f1ec))
* Release 1.7.10 ([#52](https://github.com/dasch-swiss/ark-resolver/issues/52)) ([4819f10](https://github.com/dasch-swiss/ark-resolver/commit/4819f10aeef5d5c2b47e085e1dc596b83c8d38d4))
* Release 1.7.8 ([#47](https://github.com/dasch-swiss/ark-resolver/issues/47)) ([cd41421](https://github.com/dasch-swiss/ark-resolver/commit/cd414217d864ade5eb84ecb81d6fe65961ee982e))
* Release 1.7.9 ([#49](https://github.com/dasch-swiss/ark-resolver/issues/49)) ([78464a7](https://github.com/dasch-swiss/ark-resolver/commit/78464a7b9b13df4e51b1d8e5e16c1ee8c4fd4d58))
* Revert all release-please related changes (DEV-2705) ([#55](https://github.com/dasch-swiss/ark-resolver/issues/55)) ([11962dc](https://github.com/dasch-swiss/ark-resolver/commit/11962dcc5df11cfa2eb2e984f28fc1fe10c41a5d))
* Split ark.py into modules. ([65505c8](https://github.com/dasch-swiss/ark-resolver/commit/65505c81ef3def9975c1cecf0ef815a4792c0246))


### Documentation

* Update README. ([0462a04](https://github.com/dasch-swiss/ark-resolver/commit/0462a04a0be21e96205a516856e6d61c383469d8))


### Enhancements

* Add convenience functionality for generating custom resource IRIs. ([3dda56c](https://github.com/dasch-swiss/ark-resolver/commit/3dda56c59fd8255ac9dc6d06a05ccc77efe19661))
* Clarify error message. ([8fc3e96](https://github.com/dasch-swiss/ark-resolver/commit/8fc3e968dfe3bb71011aeadde3bdd9219fb875ec))
* **config:** Change default URL templates to point to GUI pages. ([9bc5ea3](https://github.com/dasch-swiss/ark-resolver/commit/9bc5ea3272a6738b63e95020af640773b83899b4))
* Support ARK URLs for values as per https://github.com/dhlab-basel/Knora/pull/1322 ([b6197b3](https://github.com/dasch-swiss/ark-resolver/commit/b6197b3b26ae3ae225b252fb2d20c21f30a09be9))


### Bug Fixes

* Add PR title constraint in order to make release-please work ([#45](https://github.com/dasch-swiss/ark-resolver/issues/45)) ([862bced](https://github.com/dasch-swiss/ark-resolver/commit/862bced1394e8087b624db6bb7ad67fe5ed3abff))
* Change docker image to start as server ([32bb0e2](https://github.com/dasch-swiss/ark-resolver/commit/32bb0e277068ddf20037ea2ad4dea747c623828b))
* Check GitHub signature in webhook route. ([1795607](https://github.com/dasch-swiss/ark-resolver/commit/17956079bb7b81dc1185e8f0f3607e2ae86444f0))
* Deployment action needs a token with more permissions ([#48](https://github.com/dasch-swiss/ark-resolver/issues/48)) ([62f0893](https://github.com/dasch-swiss/ark-resolver/commit/62f0893040ce6584e2261877b7414893e6e2cb29))
* Fix ark resolver not working ([#59](https://github.com/dasch-swiss/ark-resolver/issues/59)) ([6212edb](https://github.com/dasch-swiss/ark-resolver/commit/6212edbfdd369bea09b23cc90748f3f0874a2974))
* Fix publishing to dockerhub job: trigger when a release was published ([#51](https://github.com/dasch-swiss/ark-resolver/issues/51)) ([df0c1d1](https://github.com/dasch-swiss/ark-resolver/commit/df0c1d1703830809fec812f331ff3d309525885f))
* Fix requirements.txt. ([d054ef1](https://github.com/dasch-swiss/ark-resolver/commit/d054ef1f8bad736a665679a81daba737891a6165))
* Fix secret not being removed from /config response ([#58](https://github.com/dasch-swiss/ark-resolver/issues/58)) ([0dafdd9](https://github.com/dasch-swiss/ark-resolver/commit/0dafdd9631974a714c951e7905a9c2653a1a222d))
* Invalid project ARK (DEV-1536) ([b47c1a7](https://github.com/dasch-swiss/ark-resolver/commit/b47c1a70e6669f5b9f21c9408f1a5d24fc169ae3))
* **release-please:** Run tests on pull_request instead of on push ([#53](https://github.com/dasch-swiss/ark-resolver/issues/53)) ([1e58ce0](https://github.com/dasch-swiss/ark-resolver/commit/1e58ce00195b4839444e2bfcf17d3cafe46b40ef))
* **release-please:** Use secrets.GITHUB_TOKEN instead of secrets.GH_TOKEN ([#46](https://github.com/dasch-swiss/ark-resolver/issues/46)) ([a60f837](https://github.com/dasch-swiss/ark-resolver/commit/a60f837e150ba0cf27f2f16883fff9ddc6111421))
* Support HTTP method HEAD for URL /config ([#10](https://github.com/dasch-swiss/ark-resolver/issues/10)) ([2a92692](https://github.com/dasch-swiss/ark-resolver/commit/2a92692271fae0f4e20ac68b288875577d4c3877))
* Support project ARK URLs for projects that are hosted on salsah.org. ([3b0a1b0](https://github.com/dasch-swiss/ark-resolver/commit/3b0a1b06af199ee521981f227c1fd16899ebd4c2))


### Tests

* Fix tests. ([50d7df5](https://github.com/dasch-swiss/ark-resolver/commit/50d7df581a4d2c70808311b093aae26e9f00e0a3))
