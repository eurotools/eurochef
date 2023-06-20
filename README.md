# 👨‍🍳 Eurochef

_Cooking up some EDBs_

Eurochef provides tools and Rust crates for working with Eurocom EngineX(T) files, including filelist, .edb, .sfx and .elx files.

## Features

* [x] Easy to use CLI Tool
* [x] Texture extractor
  * Supported output formats: png, qoi, tga
* [x] Entity extractor
* [x] Map extractor
  * [x] Blender plugin
* [x] Filelist re-packer
* [x] GUI viewer tool (WIP)
* [x] Filelist VFS
* [x] Intermediate representation of EDB files
* [x] EDB to Euroland 4 decompiler
* [x] And more?

## Support Matrix

### Games (EDB)

_(Priority currently lies with Spyro and G-Force)_
| Game (EDB Version)                             | Textures <sup>[1]</sup> | Maps | Scripts | Entities | Animations | Particles | Spreadsheets |
| ---------------------------------------------- | ----------------------- | ---- | ------- | -------- | ---------- | --------- | ------------ |
| Sphinx and the Shadow of Set Demo Disc (156)   | ✅/❌                     | 🚧/❌    | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Sphinx and the Cursed Mummy (182)              | ✅/❌                     | 🚧/❌    | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Spyro: A Hero's Tail (240)                     | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Robots (248)                                   | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Predator: Concrete Jungle (250)                | ✅/❌                     | ❔/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Batman Begins (251)                            | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Ice Age 2: The Meltdown (252)                  | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Pirates of the Caribbean: At World's End (252) | ✅/❌                     | ✅/❌  | 🚧/❌       | 🚧/❌        | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Ice Age: Dawn of the Dinosaurs (260)           | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| G-Force (259)                                  | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| Spiderman 4 (263)                              | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |
| GoldenEye 007 (263)                            | ✅/❌                     | ✅/❌  | 🚧/❌       | ✅/❌      | 🚧/❌          | 🚧/❌         | ✅/❌          |

<sup>[1]</sup> Texture/entity support only indicates the ability to read headers and frame data. See the platform matrix for texture/mesh encoding/decoding support

_❔ indicates an untested feature_

_Each field is formatted as R/W. For example, if a feature can be read, but not written, the field would be ✅/❌. If a feature can be both/neither read and/or written it will be represented by a single icon instead_

### Platforms

| Platform      | Endian | Textures          | Sounds | Mesh              | Support status<sup>[4]</sup> |
| ------------- | ------ | ----------------- | ------ | ----------------- | ---------------------------- |
| PC            | LE     | ✅<sup>[2]</sup>/❌ | 🚧/❌      | ✅/❌               | ✅                            |
| Xbox          | LE     | ✅<sup>[2]</sup>/❌ | 🚧/❌      | ✅/❌               | ✅                            |
| Xbox 360      | BE     | ✅<sup>[2]</sup>/❌ | 🚧/❌      | ✅/❌               | 🆗                            |
| GameCube      | BE     | ✅<sup>[2]</sup>/❌ | 🚧/❌      | ✅/❌               | 🆗                            |
| Wii           | BE     | ✅<sup>[2]</sup>/❌ | 🚧/❌      | ✅/❌               | 🆗                            |
| Wii U         | BE     | 🚧/❌                 | 🚧/❌      | 🚧/❌                 | 🚧/❌                            |
| Playstation 2 | LE     | ✅<sup>[2]</sup>/❌ | 🚧/❌      | 🚧<sup>[3]</sup>/❌ | 🆗                            |
| Playstation 3 | BE     | 🚧/❌                 | 🚧/❌      | 🚧/❌                 | 🚧/❌                            |

<sup>[2]</sup> The most significant formats have been implemented, no games using the remaining formats are currently known

<sup>[3]</sup> Currently has broken triangle strips, and no transparency information/flags.

<sup>[4]</sup> ✅ = First class support 🆗 = Secondary support ❌ = Unsupported

### Filelists

| Version | Read | Write |
| ------- | ---- | ----- |
| v4      | ✅    | ✅     |
| v5      | ✅    | ✅     |
| v6      | ✅    | ✅     |
| v7      | ✅    | ✅     |
| v9      | ✅    | ✅     |
| v10     | ✅    | ✅     |
| v11     | ✅    | ✅     |
| v12     | ✅    | ✅     |
| v13     | ✅    | ✅     |

<!-- ## Map extracting -->
<!-- TODO(cohae): Write this out into a guide on how to build/use CLI/GUI, not just for maps but also everything else -->
