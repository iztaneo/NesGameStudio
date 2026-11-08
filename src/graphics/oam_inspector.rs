/// Inspector y decodificador de la Object Attribute Memory (`OAM`, 64 sprites = 256 bytes) de la PPU NES (Fase G3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OamEntry {
    pub oam_index: u8,
    pub x: u8,
    pub y: u8,
    pub tile_id: u8,
    pub attributes: u8,
    /// Paleta de sprite (0..3), mapeada a `palette_table[16 + palette_idx * 4 .. 16 + palette_idx * 4 + 4]`.
    pub palette_idx: u8,
    /// `true` si el sprite se dibuja detrás del fondo (bit 5 de `attributes`).
    pub behind_bg: bool,
    /// Espejado horizontal (bit 6 de `attributes`).
    pub flip_h: bool,
    /// Espejado vertical (bit 7 de `attributes`).
    pub flip_v: bool,
    /// `true` cuando el bit 5 de `PPUCTRL` (`0x20`) activa sprites de `8x16` píxeles.
    pub is_8x16: bool,
    /// Índice de tile superior en la ventana de 8 KB de VRAM (`0..511`).
    pub top_vram_tile: u16,
    /// Índice de tile inferior en la ventana de 8 KB de VRAM (`0..511`), presente en modo `8x16`.
    pub bottom_vram_tile: Option<u16>,
}

impl OamEntry {
    pub fn width(&self) -> u16 {
        8
    }

    pub fn height(&self) -> u16 {
        if self.is_8x16 {
            16
        } else {
            8
        }
    }
}

/// Decodifica las 64 entradas de la tabla `OAM` (`256 bytes`) respetando los bits de `PPUCTRL`:
/// - Bit 5 (`0x20`): Tamaño de sprite (`0` = `8x8`, `1` = `8x16`).
/// - Bit 3 (`0x08`): Tabla de patrones para sprites `8x8` (`0` = `$0000`, `1` = `$1000` / tile +256).
/// - En modo `8x16`, el bit 0 de `tile_id` selecciona el banco (`$0000` o `$1000`) y `tile_id & 0xFE` es el par superior/inferior.
pub fn inspect_scene_oam(oam: &[u8; 256], ppu_ctrl_bits: u8) -> Vec<OamEntry> {
    let is_8x16 = (ppu_ctrl_bits & 0x20) != 0;
    let sprite_table_base_8x8: u16 = if (ppu_ctrl_bits & 0x08) != 0 { 256 } else { 0 };
    let mut entries = Vec::new();

    for idx in 0..64u8 {
        let base = (idx as usize) * 4;
        let raw_y = oam[base];
        let tile_id = oam[base + 1];
        let attributes = oam[base + 2];
        let x = oam[base + 3];

        // Descartar sprites ocultos fuera del área visible (Y >= 0xEF o entradas vacías en 0,0)
        if raw_y >= 0xEF || (raw_y == 0 && x == 0 && tile_id == 0 && attributes == 0) {
            continue;
        }
        let y = raw_y.saturating_add(1);

        let palette_idx = attributes & 0x03;
        let behind_bg = (attributes & 0x20) != 0;
        let flip_h = (attributes & 0x40) != 0;
        let flip_v = (attributes & 0x80) != 0;

        let (top_vram_tile, bottom_vram_tile) = if is_8x16 {
            let bank_offset: u16 = if (tile_id & 0x01) != 0 { 256 } else { 0 };
            let even_tile = (tile_id & 0xFE) as u16;
            let top = bank_offset + even_tile;
            let bottom = top + 1;
            (top, Some(bottom))
        } else {
            (sprite_table_base_8x8 + (tile_id as u16), None)
        };

        entries.push(OamEntry {
            oam_index: idx,
            x,
            y,
            tile_id,
            attributes,
            palette_idx,
            behind_bg,
            flip_h,
            flip_v,
            is_8x16,
            top_vram_tile,
            bottom_vram_tile,
        });
    }

    entries
}
