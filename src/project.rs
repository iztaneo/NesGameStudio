use nes_emu_rust::nes::Nes;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn nes_color_rgb_u8(nes_idx: u8) -> [u8; 3] {
    const TABLE: [[u8; 3]; 64] = [
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
    TABLE[(nes_idx as usize) & 63]
}

#[derive(Clone, Debug)]
pub struct SubroutineInfo {
    pub addr: u16,
    pub name: String,
    pub category: String,
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

/// Captura real de una escena del juego (con sus bancos CHR descomprimidos por el motor 6502,
/// su paleta exacta de 32 bytes, sus 64 sprites OAM, su imagen 256x240 y su estado de CPU/APU).
#[derive(Clone)]
pub struct CapturedScene {
    pub title: String,
    pub chr_8kb: Vec<u8>,
    pub palette_table: [u8; 32],
    pub oam: [u8; 256],
    pub ppu_ctrl_bits: u8,
    pub screen_rgb: Vec<u8>,
    pub save_state: Vec<u8>,
}

pub struct NgsProject {
    pub loaded: bool,
    pub game_title: String,
    pub c_file_path: Option<PathBuf>,
    pub asm_file_path: Option<PathBuf>,
    pub raw_c_source: String,
    pub mapper_id: u8,
    pub mirroring_mode: u8,
    pub is_chr_ram: bool,
    pub vec_nmi: u16,
    pub vec_reset: u16,
    pub vec_irq: u16,
    pub prg_rom: Vec<u8>,
    pub chr_mem: Vec<u8>,
    /// Escenas capturadas con los mapas de bits reales descomprimidos (Título, Nivel 1, Capturas de usuario)
    pub scenes: Vec<CapturedScene>,
    pub active_scene_idx: usize,
    /// Instancia en vivo del núcleo 6502 + PPU + APU para la Rockola y capturas interactivas
    pub nes_live: Nes,
    pub sound_cmd_ram_addrs: Vec<u16>,
    pub modified_tiles: HashMap<usize, [u8; 16]>,
    pub hd_sprites: HashMap<usize, HdSpriteBinding>,
    pub audio_replacements: HashMap<u8, AudioReplacement>,
    pub subroutines: Vec<SubroutineInfo>,
    pub selected_palette: usize, // 0..3 = Fondo 0..3, 4..7 = Sprites 0..3
    pub widescreen_16_9: bool,
}

impl Default for NgsProject {
    fn default() -> Self {
        Self {
            loaded: false,
            game_title: "(Ningún proyecto abierto)".to_string(),
            c_file_path: None,
            asm_file_path: None,
            raw_c_source: String::new(),
            mapper_id: 0,
            mirroring_mode: 0,
            is_chr_ram: false,
            vec_nmi: 0x8000,
            vec_reset: 0x8000,
            vec_irq: 0x8000,
            prg_rom: Vec::new(),
            chr_mem: Vec::new(),
            scenes: Vec::new(),
            active_scene_idx: 0,
            nes_live: Nes::new_with_demo(),
            sound_cmd_ram_addrs: Vec::new(),
            modified_tiles: HashMap::new(),
            hd_sprites: HashMap::new(),
            audio_replacements: HashMap::new(),
            subroutines: Vec::new(),
            selected_palette: 4, // Por defecto Paleta de Sprites #0
            widescreen_16_9: false,
        }
    }
}

impl NgsProject {
    pub fn clear_to_new_project(&mut self) {
        *self = Self::default();
        self.loaded = true;
        self.game_title = "Nuevo_Juego_NGS".to_string();
        self.scenes.push(CapturedScene {
            title: "Banco Inicial (512 Tiles en Blanco)".to_string(),
            chr_8kb: vec![0u8; 8192],
            palette_table: [
                0x0F, 0x00, 0x10, 0x30, 0x0F, 0x16, 0x27, 0x30, 0x0F, 0x19, 0x29, 0x39, 0x0F, 0x01, 0x11, 0x21,
                0x0F, 0x16, 0x27, 0x30, 0x0F, 0x12, 0x28, 0x38, 0x0F, 0x14, 0x24, 0x34, 0x0F, 0x06, 0x16, 0x26,
            ],
            oam: [0xFF; 256],
            ppu_ctrl_bits: 0,
            screen_rgb: vec![0u8; 256 * 240 * 3],
            save_state: Vec::new(),
        });
    }

    pub fn active_chr_slice(&self) -> &[u8] {
        if let Some(sc) = self.scenes.get(self.active_scene_idx) {
            &sc.chr_8kb
        } else {
            &self.chr_mem
        }
    }

    /// Devuelve los 4 colores NES de la paleta activa (0..3 = Fondo, 4..7 = Sprites) leídos de la escena real
    pub fn active_palette_4(&self, pal_idx: usize) -> [u8; 4] {
        let p = pal_idx & 7;
        if let Some(sc) = self.scenes.get(self.active_scene_idx) {
            let bg0 = sc.palette_table[0];
            let base = p * 4;
            [
                bg0,
                sc.palette_table[base + 1],
                sc.palette_table[base + 2],
                sc.palette_table[base + 3],
            ]
        } else {
            [0x0F, 0x16, 0x27, 0x30]
        }
    }

    pub fn total_tiles(&self) -> usize {
        self.active_chr_slice().len() / 16
    }

    pub fn get_tile_pixels(&self, tile_idx: usize) -> [[u8; 8]; 8] {
        let mut out = [[0u8; 8]; 8];
        let chr = self.active_chr_slice();
        let base = tile_idx * 16;
        if base + 16 > chr.len() {
            return out;
        }
        for row in 0..8 {
            let p0 = chr[base + row];
            let p1 = chr[base + row + 8];
            for col in 0..8 {
                let bit = 7 - col;
                let lo = (p0 >> bit) & 1;
                let hi = (p1 >> bit) & 1;
                out[row][col] = (hi << 1) | lo;
            }
        }
        out
    }

    pub fn set_tile_pixel(&mut self, tile_idx: usize, row: usize, col: usize, color_idx: u8) {
        let Some(sc) = self.scenes.get_mut(self.active_scene_idx) else {
            return;
        };
        let base = tile_idx * 16;
        if base + 16 > sc.chr_8kb.len() || row >= 8 || col >= 8 {
            return;
        }
        let bit = 7 - col;
        let lo = color_idx & 1;
        let hi = (color_idx >> 1) & 1;

        let mut p0 = sc.chr_8kb[base + row] & !(1 << bit);
        let mut p1 = sc.chr_8kb[base + row + 8] & !(1 << bit);
        p0 |= lo << bit;
        p1 |= hi << bit;

        sc.chr_8kb[base + row] = p0;
        sc.chr_8kb[base + row + 8] = p1;

        let mut tile_bytes = [0u8; 16];
        tile_bytes.copy_from_slice(&sc.chr_8kb[base..base + 16]);
        self.modified_tiles.insert(tile_idx, tile_bytes);

        // Actualizar también en la instancia en vivo si tiene cartucho
        if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
            if base + 16 <= cart.chr_mem.len() {
                cart.chr_mem[base..base + 16].copy_from_slice(&tile_bytes);
            }
        }
    }

    pub fn capture_current_live_scene(&mut self, custom_title: String) {
        let chr_8kb = self
            .nes_live
            .bus
            .cartridge
            .as_ref()
            .map(|c| c.chr_mem.clone())
            .unwrap_or_else(|| vec![0u8; 8192]);
        let palette_table = self.nes_live.bus.ppu.palette_table;
        let oam = self.nes_live.bus.ppu.oam;
        let ppu_ctrl_bits = self.nes_live.bus.ppu.ctrl.bits();
        let screen_rgb = self.nes_live.bus.ppu.framebuffer.clone();
        let save_state = self.nes_live.to_save_state_bytes();

        self.scenes.push(CapturedScene {
            title: custom_title,
            chr_8kb,
            palette_table,
            oam,
            ppu_ctrl_bits,
            screen_rgb,
            save_state,
        });
        self.active_scene_idx = self.scenes.len() - 1;
    }

    pub fn save_project_manifest(&self, path: &Path) -> Result<(), String> {
        let mut lines = Vec::new();
        lines.push("{".to_string());
        lines.push(format!("  \"game_title\": \"{}\",", self.game_title));
        lines.push(format!("  \"mapper_id\": {},", self.mapper_id));
        lines.push(format!("  \"widescreen_16_9\": {},", self.widescreen_16_9));
        lines.push(format!("  \"scenes_captured\": {},", self.scenes.len()));
        lines.push(format!("  \"modified_tiles_count\": {},", self.modified_tiles.len()));
        lines.push(format!("  \"hd_sprites_count\": {},", self.hd_sprites.len()));
        lines.push(format!("  \"audio_replacements_count\": {}", self.audio_replacements.len()));
        lines.push("}".to_string());
        fs::write(path, lines.join("\n")).map_err(|e| e.to_string())
    }
}
