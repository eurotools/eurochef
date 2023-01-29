# 👨‍🍳 Eurochef

_Cooking up some EDBs_

Eurochef provides tools and Rust crates for working with Eurocom EngineX(T) files, including .edb, .sfx and .elx files.

## Features

- [ ] Proper CLI Tool
- [ ] Intermediate representation of EDB files
- [ ] And more?

## Support Matrix

### Game matrix

_(Priority currently lies with G-Force)_

| Game (version)                                 | Textures <sup>[1]</sup> | Maps | Scripts | Entities | Animations | Particles | Spreadsheets |
| ---------------------------------------------- | ----------------------- | ---- | ------- | -------- | ---------- | --------- | ------------ |
| Sphinx and the Cursed Mummy (182)              | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | ❌           |
| Spyro: A Hero's Tail (240)                     | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | 🚧/❌        |
| Ice Age 2: The Meltdown (252)                  | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | ❌           |
| Predator: Concrete Jungle (250)                | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | ❌           |
| Pirates of the Caribbean: At World's End (252) | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | ❌           |
| Ice Age: Dawn of the Dinosaurs (258/260)       | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | 🚧           |
| G-Force (259)                                  | 🚧/❌                   | ❌   | ❌      | ❌       | ❌         | ❌        | ✅/❌        |
| GoldenEye 007 (263)                            | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | ✅/❌        |

<!-- ### Game matrix (unreleased builds)

| Game (version)                       | Textures <sup>[1]</sup> | Maps | Scripts | Entities | Animations | Particles | Spreadsheets |
| ------------------------------------ | ----------------------- | ---- | ------- | -------- | ---------- | --------- | ------------ |
| Spyro: A Hero's Tail (E3 Demo) (213) | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | 🚧/❌        |
| Spyro: A Hero's Tail (??) (221)      | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | 🚧/❌        |
| Spyro: A Hero's Tail (??) (236)      | ❌                      | ❌   | ❌      | ❌       | ❌         | ❌        | 🚧/❌        | -->

<sup>[1]</sup> Texture support only indicates the ability to read texture headers and frame data. See the platform matrix for texture encoding/decoding support

_Each field is formatted as R/W. For example, if a feature can be read, but not written, the field would be ✅/❌. If a feature can be both/neither read or written it will be represented by a single icon instead_

### Platform matrix

| Platform      | Endian | Textures | Sounds |
| ------------- | ------ | -------- | ------ |
| PC            | LE     | 🚧/❌    | ❌     |
| Xbox          | LE     | 🚧/❌    | ❌     |
| Xbox 360      | BE     | ❌       | ❌     |
| GameCube      | BE     | ❌       | ❌     |
| Wii           | BE     | ❌       | ❌     |
| Wii U         | BE     | ❌       | ❌     |
| Playstation 2 | LE     | ❌       | ❌     |
| Playstation 3 | BE     | ❌       | ❌     |
