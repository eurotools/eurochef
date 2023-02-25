# Texture formats

This document contains all EXTexFmt format mappings for every platform.

Pointers to implementation and documentation for these formats will be given where possible

## PS2

### EXTexFmt

| # | Format  | Internal ID |
|---|---------|-------------|
| 0 | P16x16  | 0x14        |
| 1 | P16x32  | 0x14        |
| 2 | P256x16 | 0x13        |
| 3 | P256x32 | 0x13        |
| 4 | 16BIT   | 0xa         |
| 5 | 32BIT   | 0x0         |

Additionally, EngineX has a table for CLUT formats for the PS2 platform, named `EXClutFmt` (*this doesnt appear to be actually used anywhere?*)

### EXClutFmt

| #  | Format | Internal ID |
|----|--------|-------------|
| 0  | 16BIT  | 0xa         |
| 1  | 32BIT  | 0x0         |
| 2  | 32BIT  | 0x0         |
| 3  | 32BIT  | 0x0         |
| 4  | 16BIT  | 0xa         |
| 5  | 32BIT  | 0x0         |
| 6  | 32BIT  | 0x0         |
| 7  | 32BIT  | 0x0         |
| 8  | 32BIT  | 0x0         |
| 9  | 32BIT  | 0x0         |
| 10 | 32BIT  | 0x0         |
| 11 | 32BIT  | 0x0         |


## GameCube (/Wii?)

### EXTexFmt


:warning: GameCube and Wii don't encode texture data linearly. Instead, they use a [block format](https://wiki.tockdom.com/wiki/Image_Formats#Blocks)

:information_source: There are more formats than the ones listed below. Eurochef relies on an extra header containing the raw GX format ID instead.

[Texture format documentation](https://wiki.tockdom.com/wiki/Image_Formats)


| # | Format | Internal ID |
|---|--------|-------------|
| 0 | CMPR   | 0xe         |
| 1 | RGBA8  | 0x6         |
| 3 | RGB5A3 | 0x5         |
| 4 | I4     | 0x0         |
| 7 | IA4    | 0x2         |
| 8 | IA8    | 0x3         |

## Xbox

### EXTexFmt

:warning: [Some formats are swizzled.](https://github.com/Cxbx-Reloaded/Cxbx-Reloaded/blob/master/src/core/hle/D3D8/XbD3D8Types.h#L116)

| # | Format                                 | Internal ID |
|---|----------------------------------------|-------------|
| 0 | R5G6B5                                 | 0x5         |
| 1 | X1R5G5B5                               | 0x3         |
| 2 | DXT1                                   | 0xc         |
| 3 | DXT1                                   | 0xc         |
| 4 | DXT2                                   | 0xe         |
| 5 | A4R4G4B4                               | 0x4         |
| 6 | A8R8G8B8                               | 0x6         |
| 7 | PAL8D3DFMT_A1R5G5B5D3DFMT_LIN_A8R8G8B8 | 0xb         |

With Ice Age 2: The Meltdown, support for 5 more formats was added

| #  | Format            | Internal ID |
|----|-------------------|-------------|
| 8  | D3DFMT_A1R5G5B5   | 0x2         |
| 9  | A8R8G8B8 (Linear) | 0x12        |
| 10 | DXT3              | 0xe         |
| 11 | DXT4              | 0xf         |
| 12 | DXT5              | 0xf         |

## PC

| # | Format   | Internal ID |
|---|----------|-------------|
| 0 | R5G6B5   | 0x17        |
| 1 | A1R5G5B5 | 0x19        |
| 2 | DXT1     | 'DXT1'      |
| 3 | DXT1     | 'DXT1'      |
| 4 | DXT2     | 'DXT2'      |
| 5 | A4R4G4B4 | 0x1a        |
| 6 | A8R8G8B8 | 0x15        |
| 7 | DXT3     | 'DXT3'      |
| 8 | DXT4     | 'DXT4'      |
| 9 | DXT5     | 'DXT5'      |
