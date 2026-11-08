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

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AudioTrigger {
    Subroutine(u16),
    RamAddress(u16),
    DriverCommand {
        entry_point: u16,
        queue_address: u16,
    },
    Unknown,
}

impl AudioTrigger {
    pub fn short_label(&self) -> String {
        match self {
            AudioTrigger::Subroutine(addr) => format!("JSR ${:04X}", addr),
            AudioTrigger::RamAddress(addr) => format!("RAM ${:04X}", addr),
            AudioTrigger::DriverCommand {
                entry_point,
                queue_address,
            } => format!("DRV ${:04X}->${:04X}", entry_point, queue_address),
            AudioTrigger::Unknown => "Desconocido".to_string(),
        }
    }
}

pub type SoundTrigger = AudioTrigger;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct AudioEventKey {
    pub trigger: AudioTrigger,
    pub command_id: u8,
    pub bank_context: Option<u16>,
}

impl AudioEventKey {
    pub fn new(trigger: AudioTrigger, command_id: u8) -> Self {
        Self {
            trigger,
            command_id,
            bank_context: None,
        }
    }

    #[allow(dead_code)]
    pub fn with_bank(trigger: AudioTrigger, command_id: u8, bank: u16) -> Self {
        Self {
            trigger,
            command_id,
            bank_context: Some(bank),
        }
    }

    pub fn short_label(&self) -> String {
        match self.bank_context {
            Some(b) => format!("{} :: ${:02X} (Bank ${:02X})", self.trigger.short_label(), self.command_id, b),
            None => format!("{} :: ${:02X}", self.trigger.short_label(), self.command_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioEventKind {
    Music,
    Sfx,
    Fanfare,
    Control,
    Unknown,
}

impl AudioEventKind {
    pub fn is_music_like(&self) -> bool {
        matches!(self, AudioEventKind::Music | AudioEventKind::Fanfare)
    }

    pub fn label_es(&self) -> &'static str {
        match self {
            AudioEventKind::Music => "Música (BGM)",
            AudioEventKind::Sfx => "Efecto (SFX)",
            AudioEventKind::Fanfare => "Fanfarria",
            AudioEventKind::Control => "Control / Silencio",
            AudioEventKind::Unknown => "Sin clasificar",
        }
    }

    pub fn label(&self) -> &'static str {
        self.label_es()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AudioValidationStatus {
    Confirmed,
    Candidate,
    Unclassified,
    Rejected(String),
}

impl AudioValidationStatus {
    pub fn label_es(&self) -> &'static str {
        match self {
            AudioValidationStatus::Confirmed => "Confirmado",
            AudioValidationStatus::Candidate => "Candidato",
            AudioValidationStatus::Unclassified => "Sin clasificar",
            AudioValidationStatus::Rejected(_) => "Rechazado",
        }
    }

    pub fn label(&self) -> &'static str {
        self.label_es()
    }
}

#[derive(Clone, Debug)]
pub struct DiscoveredTrack {
    pub key: AudioEventKey,
    pub kind: AudioEventKind,
    pub status: AudioValidationStatus,
    pub id: u8,
    pub title: String,
    pub is_music: bool,
    pub approx_frames: u16,
    pub active_channels: [bool; 5],
    pub confidence: f32,
}

impl DiscoveredTrack {
    pub fn new_event(
        key: AudioEventKey,
        kind: AudioEventKind,
        status: AudioValidationStatus,
        title: impl Into<String>,
    ) -> Self {
        let id = key.command_id;
        let is_music = kind.is_music_like();
        Self {
            key,
            kind,
            status,
            id,
            title: title.into(),
            is_music,
            approx_frames: 0,
            active_channels: [false; 5],
            confidence: 0.5,
        }
    }

    pub fn with_key(
        key: AudioEventKey,
        kind: AudioEventKind,
        status: AudioValidationStatus,
        title: impl Into<String>,
    ) -> Self {
        Self::new_event(key, kind, status, title)
    }

    pub fn with_metrics(
        mut self,
        approx_frames: u16,
        active_channels: [bool; 5],
        confidence: f32,
    ) -> Self {
        self.approx_frames = approx_frames;
        self.active_channels = active_channels;
        self.confidence = confidence;
        self
    }

    pub fn set_kind(&mut self, kind: AudioEventKind) {
        self.is_music = kind.is_music_like();
        self.kind = kind;
    }

    pub fn channels_badge(&self) -> String {
        let labels = ["P1", "P2", "Tri", "Noi", "DMC"];
        let active: Vec<&str> = labels
            .iter()
            .enumerate()
            .filter_map(|(i, &l)| if self.active_channels[i] { Some(l) } else { None })
            .collect();
        if active.is_empty() {
            "-".to_string()
        } else {
            active.join("+")
        }
    }
}

#[derive(Clone, Debug)]
pub struct AudioReplacement {
    pub event_key: Option<AudioEventKey>,
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
#[allow(dead_code)]
pub struct CapturedScene {
    pub title: String,
    pub chr_8kb: Vec<u8>,
    pub palette_table: [u8; 32],
    pub oam: [u8; 256],
    pub ppu_ctrl_bits: u8,
    pub screen_rgb: Vec<u8>,
    pub save_state: Vec<u8>,
}

#[derive(Clone, Debug)]
pub struct GlobalTile {
    pub bytes: [u8; 16],
    pub label: String,
    pub category: String,
    pub scene_idx: usize,
    pub tile_in_scene: usize,
    pub prg_offset: Option<usize>,
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
    /// Catálogo Global con TODOS los tiles del juego en una sola lista continua
    pub global_tiles: Vec<GlobalTile>,
    /// Escenas capturadas para el motor de audio y estados de juego
    pub scenes: Vec<CapturedScene>,
    pub active_scene_idx: usize,
    /// Instancia en vivo del núcleo 6502 + PPU + APU para la Rockola y capturas interactivas
    pub nes_live: Nes,
    pub sound_cmd_ram_addrs: Vec<u16>,
    pub sound_trigger: SoundTrigger,
    pub audio_triggers: Vec<AudioTrigger>,
    pub discovered_tracks: Vec<DiscoveredTrack>,
    pub base_audio_save_state: Option<Vec<u8>>,
    pub modified_tiles: HashMap<usize, [u8; 16]>,
    pub hd_sprites: HashMap<usize, HdSpriteBinding>,
    pub audio_replacements: HashMap<u8, AudioReplacement>,
    pub audio_event_replacements: HashMap<AudioEventKey, AudioReplacement>,
    pub audio_track_states: HashMap<u8, Vec<u8>>,
    pub audio_event_states: HashMap<AudioEventKey, Vec<u8>>,
    pub subroutines: Vec<SubroutineInfo>,
    pub palettes: [[u8; 4]; 8],
    pub selected_palette: usize,
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
            global_tiles: Vec::new(),
            scenes: Vec::new(),
            active_scene_idx: 0,
            nes_live: Nes::new_with_demo(),
            sound_cmd_ram_addrs: Vec::new(),
            sound_trigger: SoundTrigger::Unknown,
            audio_triggers: Vec::new(),
            discovered_tracks: Vec::new(),
            base_audio_save_state: None,
            modified_tiles: HashMap::new(),
            hd_sprites: HashMap::new(),
            audio_replacements: HashMap::new(),
            audio_event_replacements: HashMap::new(),
            audio_track_states: HashMap::new(),
            audio_event_states: HashMap::new(),
            subroutines: Vec::new(),
            palettes: [
                [0x0F, 0x16, 0x27, 0x30], // 0: Paleta Cálida / Rojos
                [0x0F, 0x12, 0x28, 0x38], // 1: Paleta Fría / Azules
                [0x0F, 0x16, 0x28, 0x38], // 2: Paleta Fuego / Naranjas
                [0x0F, 0x19, 0x29, 0x39], // 3: Paleta Naturaleza / Verdes
                [0x0F, 0x01, 0x11, 0x21], // 4: Paleta Metálica / Grises
                [0x0F, 0x06, 0x16, 0x26], // 5: Paleta Tierra / Marrones
                [0x0F, 0x00, 0x10, 0x30], // 6: Monocromo / HUD
                [0x0F, 0x14, 0x24, 0x34], // 7: Paleta Mágica / Violeta
            ],
            selected_palette: 1,
            widescreen_16_9: false,
        }
    }
}

impl NgsProject {
    pub fn clear_to_new_project(&mut self) {
        *self = Self::default();
        self.loaded = true;
        self.game_title = "Nuevo_Juego_NGS".to_string();
        self.global_tiles.clear();
        for i in 0..512 {
            self.global_tiles.push(GlobalTile {
                bytes: [0u8; 16],
                label: if i < 256 {
                    format!("Sprites — Tile #{:02X}", i)
                } else {
                    format!("Escenario — Tile #{:02X}", i - 256)
                },
                category: "Proyecto en Blanco".to_string(),
                scene_idx: 0,
                tile_in_scene: i,
                prg_offset: None,
            });
        }
    }

    /// Reconstruye el catálogo unificado con TODOS los gráficos del juego:
    /// 1. Gráficos en vivo VRAM (descomprimidos dinámicamente por la PPU/CPU)
    /// 2. Gráficos de bancos CHR-ROM (si el cartucho tiene CHR-ROM)
    /// 3. TODOS los bancos de PRG-ROM (16 KB por banco para cualquier juego)
    pub fn rebuild_global_tile_catalog(&mut self) {
        self.global_tiles.clear();

        // 1. Gráficos en vivo VRAM (descomprimidos dinámicamente por la PPU/CPU)
        if !self.scenes.is_empty() {
            let mut scene_order: Vec<usize> = (0..self.scenes.len()).collect();
            scene_order.sort_by_key(|&idx| {
                let non_zero = self.scenes[idx].chr_8kb.iter().filter(|&&b| b != 0).count();
                std::cmp::Reverse(non_zero)
            });

            for &sc_idx in &scene_order {
                let sc = &self.scenes[sc_idx];
                let is_in_game = sc.title.contains("En Juego") || sc.title.contains("Nivel");
                let is_title = sc.title.contains("Título") || sc.title.contains("Inicio");

                let tile_count = sc.chr_8kb.len() / 16;
                for t_idx in 0..tile_count {
                    let base = t_idx * 16;
                    let mut b = [0u8; 16];
                    b.copy_from_slice(&sc.chr_8kb[base..base + 16]);

                    let (category, label) = if is_in_game {
                        if t_idx < 256 {
                            (
                                "Tabla de Sprites (0x0000)".to_string(),
                                format!("Sprite #{:02X}", t_idx),
                            )
                        } else {
                            (
                                "Tabla de Fondo (0x1000)".to_string(),
                                format!("Tile Fondo #{:02X}", t_idx - 256),
                            )
                        }
                    } else if is_title {
                        if t_idx < 256 {
                            (
                                "Pantalla de Inicio — Sprites".to_string(),
                                format!("Inicio Sprite #{:02X}", t_idx),
                            )
                        } else {
                            (
                                "Pantalla de Inicio — Fondo".to_string(),
                                format!("Inicio Fondo #{:02X}", t_idx - 256),
                            )
                        }
                    } else {
                        (
                            sc.title.clone(),
                            format!("{} #{:02X}", sc.title, t_idx),
                        )
                    };

                    self.global_tiles.push(GlobalTile {
                        bytes: b,
                        label,
                        category,
                        scene_idx: sc_idx,
                        tile_in_scene: t_idx,
                        prg_offset: None,
                    });
                }
            }
        }

        // 2. Gráficos de bancos CHR-ROM (si no es CHR-RAM)
        if !self.is_chr_ram && !self.chr_mem.is_empty() {
            let total = self.chr_mem.len() / 16;
            for t_idx in 0..total {
                let base = t_idx * 16;
                let mut b = [0u8; 16];
                b.copy_from_slice(&self.chr_mem[base..base + 16]);
                let bank = t_idx / 256;
                let in_b = t_idx % 256;
                self.global_tiles.push(GlobalTile {
                    bytes: b,
                    label: format!("Banco CHR #{} — Tile #{:02X}", bank, in_b),
                    category: format!("Banco CHR #{} (8 KB)", bank),
                    scene_idx: 0,
                    tile_in_scene: t_idx,
                    prg_offset: None,
                });
            }
        }

        // 3. Extracción de patrones de TODOS los bancos PRG-ROM (16 KB por banco para cualquier juego)
        let total_prg_banks = self.prg_rom.len() / 16384;
        for b_idx in 0..total_prg_banks {
            let bank_name = format!("Banco PRG #{} (16 KB)", b_idx);
            let bank_start = b_idx * 16384;
            let bank_end = bank_start + 16384;
            let bank_slice = &self.prg_rom[bank_start..bank_end];
            for t_in_b in 0..(16384 / 16) {
                let off = t_in_b * 16;
                let chunk = &bank_slice[off..off + 16];
                let is_all_zero = chunk.iter().all(|&b| b == 0);
                let is_all_ff = chunk.iter().all(|&b| b == 0xFF);
                if !is_all_zero && !is_all_ff {
                    let mut b = [0u8; 16];
                    b.copy_from_slice(chunk);
                    self.global_tiles.push(GlobalTile {
                        bytes: b,
                        label: format!("Banco PRG #{} — Tile #{:03X}", b_idx, t_in_b),
                        category: bank_name.clone(),
                        scene_idx: 0,
                        tile_in_scene: t_in_b,
                        prg_offset: Some(bank_start + off),
                    });
                }
            }
        }
    }

    pub fn total_tiles(&self) -> usize {
        self.global_tiles.len()
    }

    pub fn get_tile_pixels(&self, global_idx: usize) -> [[u8; 8]; 8] {
        let mut out = [[0u8; 8]; 8];
        let Some(t) = self.global_tiles.get(global_idx) else {
            return out;
        };
        for row in 0..8 {
            let p0 = t.bytes[row];
            let p1 = t.bytes[row + 8];
            for col in 0..8 {
                let bit = 7 - col;
                let lo = (p0 >> bit) & 1;
                let hi = (p1 >> bit) & 1;
                out[row][col] = (hi << 1) | lo;
            }
        }
        out
    }

    pub fn set_tile_pixel(&mut self, global_idx: usize, row: usize, col: usize, color_idx: u8) {
        if global_idx >= self.global_tiles.len() || row >= 8 || col >= 8 {
            return;
        }
        let bit = 7 - col;
        let lo = color_idx & 1;
        let hi = (color_idx >> 1) & 1;

        let t = &mut self.global_tiles[global_idx];
        let mut p0 = t.bytes[row] & !(1 << bit);
        let mut p1 = t.bytes[row + 8] & !(1 << bit);
        p0 |= lo << bit;
        p1 |= hi << bit;

        t.bytes[row] = p0;
        t.bytes[row + 8] = p1;

        let tile_bytes = t.bytes;
        let sc_idx = t.scene_idx;
        let in_sc = t.tile_in_scene;
        let prg_opt = t.prg_offset;

        self.modified_tiles.insert(in_sc, tile_bytes);

        if let Some(sc) = self.scenes.get_mut(sc_idx) {
            let base = in_sc * 16;
            if base + 16 <= sc.chr_8kb.len() {
                sc.chr_8kb[base..base + 16].copy_from_slice(&tile_bytes);
            }
        }

        if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
            let base = in_sc * 16;
            if base + 16 <= cart.chr_mem.len() {
                cart.chr_mem[base..base + 16].copy_from_slice(&tile_bytes);
            }
        }

        if let Some(prg_off) = prg_opt {
            if prg_off + 16 <= self.prg_rom.len() {
                self.prg_rom[prg_off..prg_off + 16].copy_from_slice(&tile_bytes);
            }
            if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                if prg_off + 16 <= cart.prg_rom.len() {
                    cart.prg_rom[prg_off..prg_off + 16].copy_from_slice(&tile_bytes);
                }
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
        lines.push(format!(
            "  \"audio_triggers_count\": {},",
            self.audio_triggers.len()
        ));
        lines.push(format!(
            "  \"audio_replacements_count\": {}",
            self.audio_event_replacements
                .len()
                .max(self.audio_replacements.len())
        ));
        lines.push("}".to_string());
        fs::write(path, lines.join("\n")).map_err(|e| e.to_string())
    }

    /// Registra un trigger de audio en el proyecto sin duplicarlo y sincroniza el trigger primario legado.
    pub fn register_audio_trigger(&mut self, trigger: AudioTrigger) {
        if matches!(trigger, AudioTrigger::Unknown) {
            return;
        }
        if !self.audio_triggers.contains(&trigger) {
            self.audio_triggers.push(trigger.clone());
        }
        if matches!(self.sound_trigger, SoundTrigger::Unknown) {
            self.sound_trigger = trigger;
        }
    }

    /// Añade o actualiza un evento de audio en `discovered_tracks` identificado por su `AudioEventKey`.
    pub fn upsert_discovered_track(&mut self, track: DiscoveredTrack) {
        self.register_audio_trigger(track.key.trigger.clone());
        if let Some(existing) = self
            .discovered_tracks
            .iter_mut()
            .find(|t| t.key == track.key)
        {
            *existing = track;
        } else {
            self.discovered_tracks.push(track);
        }
    }

    pub fn set_event_state(&mut self, key: AudioEventKey, state: Vec<u8>) {
        self.audio_track_states
            .entry(key.command_id)
            .or_insert_with(|| state.clone());
        self.audio_event_states.insert(key, state);
    }

    pub fn get_event_state(&self, key: &AudioEventKey) -> Option<&Vec<u8>> {
        self.audio_event_states
            .get(key)
            .or_else(|| self.audio_track_states.get(&key.command_id))
    }

    pub fn set_event_replacement(&mut self, key: AudioEventKey, mut replacement: AudioReplacement) {
        replacement.event_key = Some(key.clone());
        self.audio_replacements
            .insert(key.command_id, replacement.clone());
        self.audio_event_replacements.insert(key, replacement);
    }

    pub fn get_event_replacement(&self, key: &AudioEventKey) -> Option<&AudioReplacement> {
        self.audio_event_replacements
            .get(key)
            .or_else(|| self.audio_replacements.get(&key.command_id))
    }

    pub fn remove_event_replacement(&mut self, key: &AudioEventKey) {
        self.audio_event_replacements.remove(key);
        if !self
            .audio_event_replacements
            .keys()
            .any(|k| k.command_id == key.command_id)
        {
            self.audio_replacements.remove(&key.command_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_command_id_with_different_triggers_are_distinct_events() {
        let mut project = NgsProject::default();
        let bgm_trigger = AudioTrigger::RamAddress(0x04F5);
        let sfx_trigger = AudioTrigger::RamAddress(0x04F2);

        let bgm_key = AudioEventKey::new(bgm_trigger.clone(), 0x01);
        let sfx_key = AudioEventKey::new(sfx_trigger.clone(), 0x01);

        assert_ne!(
            bgm_key, sfx_key,
            "El mismo command_id ($01) con diferente AudioTrigger debe producir claves distintas"
        );

        let bgm_track = DiscoveredTrack::with_key(
            bgm_key.clone(),
            AudioEventKind::Music,
            AudioValidationStatus::Confirmed,
            "BGM $01 - Overworld",
        );
        let sfx_track = DiscoveredTrack::with_key(
            sfx_key.clone(),
            AudioEventKind::Sfx,
            AudioValidationStatus::Confirmed,
            "SFX $01 - Jump",
        );

        project.upsert_discovered_track(bgm_track);
        project.upsert_discovered_track(sfx_track);

        assert_eq!(project.discovered_tracks.len(), 2);
        assert_eq!(project.audio_triggers.len(), 2);
        assert_eq!(project.discovered_tracks[0].kind, AudioEventKind::Music);
        assert_eq!(project.discovered_tracks[1].kind, AudioEventKind::Sfx);

        project.set_event_state(bgm_key.clone(), vec![0xAA, 0xBB]);
        project.set_event_state(sfx_key.clone(), vec![0xCC, 0xDD]);
        assert_eq!(
            project.get_event_state(&bgm_key).unwrap().as_slice(),
            &[0xAA, 0xBB]
        );
        assert_eq!(
            project.get_event_state(&sfx_key).unwrap().as_slice(),
            &[0xCC, 0xDD]
        );

        project.set_event_replacement(
            bgm_key.clone(),
            AudioReplacement {
                event_key: Some(bgm_key.clone()),
                track_id: 0x01,
                label: "BGM Overworld".to_string(),
                external_file_path: "overworld.mp3".to_string(),
                is_sfx_wav: false,
            },
        );
        project.set_event_replacement(
            sfx_key.clone(),
            AudioReplacement {
                event_key: Some(sfx_key.clone()),
                track_id: 0x01,
                label: "SFX Jump".to_string(),
                external_file_path: "jump.wav".to_string(),
                is_sfx_wav: true,
            },
        );

        assert_eq!(
            project
                .get_event_replacement(&bgm_key)
                .unwrap()
                .external_file_path,
            "overworld.mp3"
        );
        assert_eq!(
            project
                .get_event_replacement(&sfx_key)
                .unwrap()
                .external_file_path,
            "jump.wav"
        );

        project.remove_event_replacement(&sfx_key);
        assert!(project.get_event_replacement(&bgm_key).is_some());
    }
}
