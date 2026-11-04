use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Paleta oficial NTSC de 64 colores de la NES (formato RGB 24-bit)
pub const NES_PALETTE_RGB: [[u8; 3]; 64] = [
    [0x66, 0x66, 0x66], [0x00, 0x2A, 0x88], [0x14, 0x12, 0xA7], [0x3B, 0x00, 0xA4],
    [0x5C, 0x00, 0x7E], [0x6E, 0x00, 0x40], [0x6C, 0x06, 0x00], [0x56, 0x1D, 0x00],
    [0x33, 0x35, 0x00], [0x0B, 0x48, 0x00], [0x00, 0x52, 0x00], [0x00, 0x4F, 0x08],
    [0x00, 0x40, 0x4D], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00],
    [0xAD, 0xAD, 0xAD], [0x15, 0x5F, 0xD9], [0x42, 0x40, 0xFF], [0x75, 0x27, 0xFE],
    [0xA0, 0x1A, 0xCC], [0xB7, 0x1E, 0x7B], [0xB5, 0x31, 0x20], [0x99, 0x4E, 0x00],
    [0x6B, 0x6D, 0x00], [0x38, 0x87, 0x00], [0x0C, 0x93, 0x00], [0x00, 0x8F, 0x32],
    [0x00, 0x7C, 0x8D], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00],
    [0xFF, 0xFE, 0xFF], [0x64, 0xB0, 0xFF], [0x92, 0x90, 0xFF], [0xC6, 0x76, 0xFF],
    [0xF3, 0x6A, 0xFF], [0xFE, 0x6E, 0xCC], [0xFE, 0x81, 0x70], [0xEA, 0x9E, 0x22],
    [0xBC, 0xBE, 0x00], [0x88, 0xD8, 0x00], [0x5C, 0xE4, 0x30], [0x45, 0xE0, 0x82],
    [0x48, 0xCD, 0xDE], [0x4F, 0x4F, 0x4F], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00],
    [0xFF, 0xFE, 0xFF], [0xC0, 0xDF, 0xFF], [0xD3, 0xD2, 0xFF], [0xE8, 0xC8, 0xFF],
    [0xFB, 0xC2, 0xFF], [0xFE, 0xC4, 0xEA], [0xFE, 0xCC, 0xC5], [0xF7, 0xD8, 0xA5],
    [0xE4, 0xE5, 0x94], [0xCF, 0xEF, 0x96], [0xBD, 0xF4, 0xAB], [0xB3, 0xF3, 0xCC],
    [0xB5, 0xEB, 0xF2], [0xB8, 0xB8, 0xB8], [0x00, 0x00, 0x00], [0x00, 0x00, 0x00],
];

#[derive(Clone, Debug)]
pub struct SubroutineInfo {
    pub addr: u16,
    pub name: String,
    pub category: String, // "[AUDIO / APU]", "[VIDEO / PPU]", "[JOYPAD / INPUT]", "[GAME LOGIC]"
    pub line_in_asm: usize,
    pub preview_asm: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct AudioReplacement {
    pub track_id: u8,
    pub label: String,
    pub external_file_path: String,
    pub is_sfx_wav: bool,
}

#[derive(Clone, Debug)]
pub struct HdSpriteBinding {
    pub tile_index: usize,
    pub width: u32,
    pub height: u32,
    pub bmp_or_rgba_path: String,
    pub rgba_pixels: Vec<u32>,
}

pub struct NgsProject {
    pub game_title: String,
    pub c_file_path: Option<PathBuf>,
    pub asm_file_path: Option<PathBuf>,
    pub raw_c_source: String,
    pub mapper_id: u8,
    pub is_chr_ram: bool,
    pub vec_nmi: u16,
    pub vec_reset: u16,
    pub vec_irq: u16,
    pub prg_rom: Vec<u8>,
    pub chr_mem: Vec<u8>,
    /// Bancos gráficos extraídos: si es CHR-ROM usa chr_mem; si es CHR-RAM extrae bloques candidatos de PRG_ROM
    pub decoded_tiles_source: Vec<u8>,
    pub modified_tiles: HashMap<usize, [u8; 16]>,
    pub hd_sprites: HashMap<usize, HdSpriteBinding>,
    pub audio_replacements: HashMap<u8, AudioReplacement>,
    pub subroutines: Vec<SubroutineInfo>,
    pub palettes: [[u8; 4]; 8],
    pub selected_palette: usize,
    pub widescreen_16_9: bool,
}

impl Default for NgsProject {
    fn default() -> Self {
        Self {
            game_title: "Nuevo_Proyecto_NGS".to_string(),
            c_file_path: None,
            asm_file_path: None,
            raw_c_source: String::new(),
            mapper_id: 0,
            is_chr_ram: false,
            vec_nmi: 0x8000,
            vec_reset: 0x8000,
            vec_irq: 0x8000,
            prg_rom: Vec::new(),
            chr_mem: Vec::new(),
            decoded_tiles_source: vec![0u8; 8192],
            modified_tiles: HashMap::new(),
            hd_sprites: HashMap::new(),
            audio_replacements: HashMap::new(),
            subroutines: Vec::new(),
            palettes: [
                [0x0F, 0x16, 0x27, 0x30], // 0: Contra / Acción Cálido
                [0x0F, 0x19, 0x29, 0x39], // 1: Selva / Verde
                [0x0F, 0x01, 0x11, 0x21], // 2: Acero / Azul Metálico
                [0x0F, 0x06, 0x16, 0x26], // 3: Fuego / Explosión
                [0x0F, 0x12, 0x28, 0x38], // 4: Héroe Jugador 1 (Azul/Piel)
                [0x0F, 0x16, 0x28, 0x38], // 5: Héroe Jugador 2 (Rojo/Piel)
                [0x0F, 0x00, 0x10, 0x30], // 6: Monocromo / Gris
                [0x0F, 0x14, 0x24, 0x34], // 7: Magia / Alien Violeta
            ],
            selected_palette: 0,
            widescreen_16_9: false,
        }
    }
}

impl NgsProject {
    pub fn total_tiles(&self) -> usize {
        self.decoded_tiles_source.len() / 16
    }

    /// Obtiene los 8x8 píxeles (valores 0..=3) de un tile
    pub fn get_tile_pixels(&self, tile_idx: usize) -> [[u8; 8]; 8] {
        let mut out = [[0u8; 8]; 8];
        let base = tile_idx * 16;
        if base + 16 > self.decoded_tiles_source.len() {
            return out;
        }
        for row in 0..8 {
            let p0 = self.decoded_tiles_source[base + row];
            let p1 = self.decoded_tiles_source[base + row + 8];
            for col in 0..8 {
                let bit = 7 - col;
                let lo = (p0 >> bit) & 1;
                let hi = (p1 >> bit) & 1;
                out[row][col] = (hi << 1) | lo;
            }
        }
        out
    }

    /// Modifica un píxel (0..=3) dentro de un tile 8x8 y lo registra para inyección C
    pub fn set_tile_pixel(&mut self, tile_idx: usize, row: usize, col: usize, color_idx: u8) {
        let base = tile_idx * 16;
        if base + 16 > self.decoded_tiles_source.len() || row >= 8 || col >= 8 {
            return;
        }
        let bit = 7 - col;
        let lo = color_idx & 1;
        let hi = (color_idx >> 1) & 1;

        let mut p0 = self.decoded_tiles_source[base + row] & !(1 << bit);
        let mut p1 = self.decoded_tiles_source[base + row + 8] & !(1 << bit);
        p0 |= lo << bit;
        p1 |= hi << bit;

        self.decoded_tiles_source[base + row] = p0;
        self.decoded_tiles_source[base + row + 8] = p1;

        let mut tile_bytes = [0u8; 16];
        tile_bytes.copy_from_slice(&self.decoded_tiles_source[base..base + 16]);
        self.modified_tiles.insert(tile_idx, tile_bytes);
    }

    /// Guarda metadatos del proyecto en archivo .ngs.json ligero
    pub fn save_project_manifest(&self, path: &Path) -> Result<(), String> {
        let mut lines = Vec::new();
        lines.push("{".to_string());
        lines.push(format!("  \"game_title\": \"{}\",", self.game_title));
        lines.push(format!("  \"mapper_id\": {},", self.mapper_id));
        lines.push(format!("  \"widescreen_16_9\": {},", self.widescreen_16_9));
        lines.push(format!("  \"modified_tiles_count\": {},", self.modified_tiles.len()));
        lines.push(format!("  \"hd_sprites_count\": {},", self.hd_sprites.len()));
        lines.push(format!("  \"audio_replacements_count\": {}", self.audio_replacements.len()));
        lines.push("}".to_string());
        fs::write(path, lines.join("\n")).map_err(|e| e.to_string())
    }
}
