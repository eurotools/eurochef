# 👨‍🍳 Eurochef

_Cooking up some EDBs_

Eurochef provides tools and Rust crates for working with Eurocom EngineX(T) files, including filelist, .edb, .sfx and .elx files.

## Features

* [x] Easy to use CLI Tool
* [x] Texture extractor
  * Supported output formats [can be found here](https://github.com/image-rs/image/blob/master/README.md#supported-image-formats)
* [x] Filelist re-packer
* [ ] Filelist VFS
* [ ] Intermediate representation of EDB files
* [ ] GUI viewer tool (Tauri+WebGL(?))
* [ ] EDB to Euroland 4 decompiler
* [ ] And more?

## Support Matrix

### Games (EDB)

_(Priority currently lies with G-Force)_

| Game (version)                                 | Textures <sup>[1]</sup> | Maps | Scripts | Entities | Animations | Particles | Spreadsheets |
|------------------------------------------------|-------------------------|------|---------|----------|------------|-----------|--------------|
| Sphinx and the Cursed Mummy (182)              | ❔/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| Spyro: A Hero's Tail (240)                     | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| Robots (248)                                   | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| Ice Age 2: The Meltdown (252)                  | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| Predator: Concrete Jungle (250)                | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| Pirates of the Caribbean: At World's End (252) | ❔/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| Ice Age: Dawn of the Dinosaurs (258/260)       | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| G-Force (259)                                  | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |
| GoldenEye 007 (263)                            | ✅/❌                     | ❌    | ❌       | ❌        | ❌          | ❌         | ✅/❌          |

<sup>[1]</sup> Texture support only indicates the ability to read texture headers and frame data. See the platform matrix for texture encoding/decoding support

_Each field is formatted as R/W. For example, if a feature can be read, but not written, the field would be ✅/❌. If a feature can be both/neither read and/or written it will be represented by a single icon instead_

### Platforms

| Platform      | Endian | Textures          | Sounds |
|---------------|--------|-------------------|--------|
| PC            | LE     | ✅<sup>[2]</sup>/❌ | ❌      |
| Xbox          | LE     | ✅<sup>[2]</sup>/❌ | ❌      |
| Xbox 360      | BE     | ❌                 | ❌      |
| GameCube      | BE     | ✅<sup>[2]</sup/❌  | ❌      |
| Wii           | BE     | ✅<sup>[2]</sup/❌  | ❌      |
| Wii U         | BE     | ❌                 | ❌      |
| Playstation 2 | LE     | ❌                 | ❌      |
| Playstation 3 | BE     | ❌                 | ❌      |

<sup>[2]</sup> The most significant formats have been implemented, no games using the remaining formats are currently known

### Filelists

| Version | Read | Write |
|---------|------|-------|
| v4      | ✅    | ❌     |
| v5      | ✅    | ✅     |
| v6      | ✅    | ✅     |
| v7      | ✅    | ✅     |
| v9      | ✅    | ❌     |
| v10     | ✅    | ❌     |
| v11     | ❌    | ❌     |
| v12     | ❌    | ❌     |
| v13     | ❌    | ❌     |

_❔ indicates an untested feature_
