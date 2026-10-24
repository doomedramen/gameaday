//! Per-biome colour palettes.

use crate::level::Biome;

#[derive(Clone, Copy, Debug)]
pub struct Pal {
    pub sky_top: u32,
    pub sky_mid: u32,
    pub sky_bot: u32,
    /// Rock light, mid, dark.
    pub rock: [u32; 3],
    pub surface: u32,
    pub surface_dark: u32,
    pub glow: u32,
    pub fog: u32,
    /// Ambient light level for dark biomes (1.0 = fully lit).
    pub ambient: f32,
    pub far: [u32; 3],
    pub accent: u32,
}

pub fn pal(b: Biome) -> Pal {
    match b {
        Biome::Mossbluff => Pal {
            sky_top: 0x4a86d6,
            sky_mid: 0x9fd0ee,
            sky_bot: 0xffd9a6,
            rock: [0x9a8c72, 0x73674f, 0x4c4333],
            surface: 0x86dc5c,
            surface_dark: 0x4aa83a,
            glow: 0xffe9a8,
            fog: 0xbfe4f2,
            ambient: 1.0,
            far: [0x8fb6d6, 0x6f9ec0, 0x4f84a6],
            accent: 0xff9a52,
        },
        Biome::Glimmerdeep => Pal {
            sky_top: 0x080818,
            sky_mid: 0x120f30,
            sky_bot: 0x241a52,
            rock: [0x5a5a92, 0x3e3e68, 0x26264a],
            surface: 0x3fe8cc,
            surface_dark: 0x1f9c8e,
            glow: 0x62f2ff,
            fog: 0x2a2468,
            ambient: 0.20,
            far: [0x2a2a62, 0x1c1c48, 0x12123a],
            accent: 0xc77dff,
        },
        Biome::Frostspire => Pal {
            sky_top: 0x8fbce8,
            sky_mid: 0xcde4f6,
            sky_bot: 0xf2f9ff,
            rock: [0xa6bcd2, 0x7f98b4, 0x5b7392],
            surface: 0xf6fcff,
            surface_dark: 0xbcd9ee,
            glow: 0xb6ecff,
            fog: 0xe4f1fb,
            ambient: 1.0,
            far: [0xc2d8ec, 0xa2c0dc, 0x86a8c8],
            accent: 0x62c8ff,
        },
        Biome::Cinderworks => Pal {
            sky_top: 0x1c0608,
            sky_mid: 0x5a1610,
            sky_bot: 0xff6a22,
            rock: [0x7a5448, 0x543630, 0x331e1c],
            surface: 0x9a6a4a,
            surface_dark: 0x5e3a2a,
            glow: 0xff9a3a,
            fog: 0x7a2210,
            ambient: 0.62,
            far: [0x5a1c14, 0x40120e, 0x2a0a08],
            accent: 0xffd24a,
        },
        Biome::Stormcrown => Pal {
            sky_top: 0x15103a,
            sky_mid: 0x3a2c70,
            sky_bot: 0x8a68a8,
            rock: [0x7270a0, 0x504e7c, 0x363458],
            surface: 0xa6a4d4,
            surface_dark: 0x6c6aa4,
            glow: 0xc8b8ff,
            fog: 0x5a4a8a,
            ambient: 1.0,
            far: [0x4a3e84, 0x362c6a, 0x241c52],
            accent: 0xffe566,
        },
        Biome::Beacon => Pal {
            sky_top: 0x0a0620,
            sky_mid: 0x2a1850,
            sky_bot: 0x70387c,
            rock: [0x6a6496, 0x484470, 0x2c2a4c],
            surface: 0xeed88a,
            surface_dark: 0xa88c48,
            glow: 0xffe9a0,
            fog: 0x4a2a6a,
            ambient: 1.0,
            far: [0x3a2c70, 0x2a2058, 0x1a1440],
            accent: 0xff7ad0,
        },
    }
}
