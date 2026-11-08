# Changelog

## [0.3.1](https://github.com/jamespero/media-offloader/compare/v0.3.0...v0.3.1) (2026-11-08)


### Bug Fixes

* **app:** align the updater plugins' npm and Rust versions ([#38](https://github.com/jamespero/media-offloader/issues/38)) ([179cb81](https://github.com/jamespero/media-offloader/commit/179cb81f06970fd759a044e3e2fd06d45e0897fd))

## [0.3.0](https://github.com/jamespero/media-offloader/compare/v0.2.0...v0.3.0) (2026-11-08)


### Features

* **app:** check for updates and install them from the app ([#22](https://github.com/jamespero/media-offloader/issues/22)) ([8e0d579](https://github.com/jamespero/media-offloader/commit/8e0d5793621d344909907002cce912b3ab48b652))
* **app:** offer to install updates automatically ([#23](https://github.com/jamespero/media-offloader/issues/23)) ([e3f3300](https://github.com/jamespero/media-offloader/commit/e3f3300fd48742baac23f71431beeea757eb7db8))
* **app:** show the real version and a repository link in About ([#20](https://github.com/jamespero/media-offloader/issues/20)) ([f7bdd36](https://github.com/jamespero/media-offloader/commit/f7bdd36404d4b4b410ffcaa9285f046018917649))

## [0.2.0](https://github.com/jamespero/media-offloader/compare/v0.1.0...v0.2.0) (2026-11-08)


### Features

* add the disk I/O layer and checksums, with offload hash and eject ([#2](https://github.com/jamespero/media-offloader/issues/2)) ([1ecdd65](https://github.com/jamespero/media-offloader/commit/1ecdd65d755a045809aac81c027a7c66d418f3f8))
* **app:** add the desktop app ([#9](https://github.com/jamespero/media-offloader/issues/9)) ([228bb01](https://github.com/jamespero/media-offloader/commit/228bb011cd0c247c7f5c5f5ca81c9ed04e9b388f))
* **app:** chime and notify when an offload ends, and confirm before quitting mid-offload ([#11](https://github.com/jamespero/media-offloader/issues/11)) ([149d7d0](https://github.com/jamespero/media-offloader/commit/149d7d0e137885ee1c0cb130775ae84e9b451ee1))
* **app:** show real thumbnails from the OS thumbnailers ([#10](https://github.com/jamespero/media-offloader/issues/10)) ([9e232cb](https://github.com/jamespero/media-offloader/commit/9e232cbf9f70de76f414ada6192f4c4052f98eab))
* copy and verify a card to a destination and backups, with offload copy and plan ([#6](https://github.com/jamespero/media-offloader/issues/6)) ([bb7dc69](https://github.com/jamespero/media-offloader/commit/bb7dc69b9a87715d1f706dee95d9ce3eff86fcc6))
* **core:** scan sources and plan every destination file ([#5](https://github.com/jamespero/media-offloader/issues/5)) ([4439a67](https://github.com/jamespero/media-offloader/commit/4439a67d3d6913a08df00c661f808fb473a27e50))
* journal every offload and resume or retry it, with offload resume and history ([#8](https://github.com/jamespero/media-offloader/issues/8)) ([ac00d0e](https://github.com/jamespero/media-offloader/commit/ac00d0e8f86bc52b51cd84461fc9ddb6c7c9d94a))
* write ASC MHL sidecars and re-verify folders, with offload verify ([#7](https://github.com/jamespero/media-offloader/issues/7)) ([a8146c6](https://github.com/jamespero/media-offloader/commit/a8146c61bdf1f6ac03c250694540293217ddad09))


### Bug Fixes

* **cli:** depend on offloader-core by path only ([#4](https://github.com/jamespero/media-offloader/issues/4)) ([c761e7b](https://github.com/jamespero/media-offloader/commit/c761e7bf638d9377e57741d63fb77100103ceac0))
