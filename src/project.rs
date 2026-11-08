use crate::graphics::{
    AutoMemoryMapper, ExtendedGraphicAsset, GraphicId, GraphicSource, TilePatch,
};
use nes_emu_rust::nes::Nes;
use std::collections::HashMap;
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
#[allow(dead_code)]
pub struct GlobalTile {
    pub id: GraphicId,
    pub original_bytes: [u8; 16],
    pub bytes: [u8; 16],
    pub label: String,
    pub category: String,
    pub scene_idx: usize,
    pub tile_in_scene: usize,
    pub prg_offset: Option<usize>,
    pub is_active_in_oam: bool,
}

pub struct NgsProject {
    pub loaded: bool,
    pub game_title: String,
    pub workspace_dir: Option<PathBuf>,
    pub workspace_template: Option<String>,
    pub base_source_rel_path: Option<String>,
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
    pub prg_rom_original: Vec<u8>,
    pub chr_mem: Vec<u8>,
    pub chr_rom_original: Vec<u8>,
    /// Catálogo limpio de tiles reales del juego (Escenas VRAM + Bancos CHR-ROM)
    pub global_tiles: Vec<GlobalTile>,
    /// Candidatos heurísticos filtrados de PRG-ROM (separados para no contaminar la vista principal)
    pub prg_candidate_tiles: Vec<GlobalTile>,
    pub show_advanced_prg_candidates: bool,
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
    /// Parches no destructivos indexados por `GraphicId` (Fase G1)
    pub tile_patches: HashMap<GraphicId, TilePatch>,
    /// Mapa legado mantenido sincronizado para compatibilidad con vistas existentes
    pub modified_tiles: HashMap<usize, [u8; 16]>,
    /// Banco de Memoria Extendida Ilimitada (Fase G1 / Fase G5)
    pub extended_assets: Vec<ExtendedGraphicAsset>,
    pub entity_custom_names: HashMap<String, String>,
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
            workspace_dir: None,
            workspace_template: None,
            base_source_rel_path: None,
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
            prg_rom_original: Vec::new(),
            chr_mem: Vec::new(),
            chr_rom_original: Vec::new(),
            global_tiles: Vec::new(),
            prg_candidate_tiles: Vec::new(),
            show_advanced_prg_candidates: false,
            scenes: Vec::new(),
            active_scene_idx: 0,
            nes_live: Nes::new_with_demo(),
            sound_cmd_ram_addrs: Vec::new(),
            sound_trigger: SoundTrigger::Unknown,
            audio_triggers: Vec::new(),
            discovered_tracks: Vec::new(),
            base_audio_save_state: None,
            tile_patches: HashMap::new(),
            modified_tiles: HashMap::new(),
            extended_assets: Vec::new(),
            entity_custom_names: HashMap::new(),
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
            let source = if i < 256 {
                GraphicSource::OamSprite
            } else {
                GraphicSource::ChrRamScene
            };
            let id = GraphicId::new(source, 0, i as u16);
            self.global_tiles.push(GlobalTile {
                id,
                original_bytes: [0u8; 16],
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
                is_active_in_oam: i < 16,
            });
        }
    }

    /// Reconstruye el catálogo gráfico separando estrictamente las fuentes reales (Fase G1 + Fase G2):
    /// 1. Gráficos activos en VRAM de las escenas capturadas (`OamSprite` y `ChrRamScene`).
    /// 2. Gráficos de bancos `CHR-ROM` reales (identificados por banco de 8 KB + índice de tile).
    /// 3. Candidatos heurísticos de `PRG-ROM` aislados en `prg_candidate_tiles` (filtrados contra código 6502)
    ///    para que `PRG_ROM` nunca contamine la vista principal de personajes.
    pub fn rebuild_global_tile_catalog(&mut self) {
        if self.prg_rom_original.is_empty() && !self.prg_rom.is_empty() {
            self.prg_rom_original = self.prg_rom.clone();
        }
        if self.chr_rom_original.is_empty() && !self.chr_mem.is_empty() {
            self.chr_rom_original = self.chr_mem.clone();
        }

        self.global_tiles.clear();
        self.prg_candidate_tiles.clear();

        // 1. Gráficos en vivo VRAM de las escenas capturadas
        if !self.scenes.is_empty() {
            let mut scene_order: Vec<usize> = (0..self.scenes.len()).collect();
            scene_order.sort_by_key(|&idx| {
                let non_zero = self.scenes[idx].chr_8kb.iter().filter(|&&b| b != 0).count();
                std::cmp::Reverse(non_zero)
            });

            let primary_sc_idx = scene_order[0];

            for &sc_idx in &scene_order {
                let sc = &self.scenes[sc_idx];
                let active_oam_indices =
                    AutoMemoryMapper::extract_active_oam_tile_indices(&sc.oam, sc.ppu_ctrl_bits);
                let is_in_game = sc.title.contains("En Juego") || sc.title.contains("Nivel");
                let is_title = sc.title.contains("Título") || sc.title.contains("Inicio");

                let tile_count = sc.chr_8kb.len() / 16;
                for t_idx in 0..tile_count {
                    let base = t_idx * 16;
                    let mut orig_b = [0u8; 16];
                    orig_b.copy_from_slice(&sc.chr_8kb[base..base + 16]);

                    let is_active_oam = active_oam_indices.contains(&(t_idx as u16));

                    // En escenas secundarias de juegos CHR-RAM, omitir tiles completamente vacíos (0x00/0xFF)
                    // que no estén referenciados por OAM para mantener el catálogo limpio.
                    if sc_idx != primary_sc_idx
                        && !is_active_oam
                        && AutoMemoryMapper::is_trivial_pattern(&orig_b)
                    {
                        continue;
                    }

                    let source = if is_active_oam {
                        GraphicSource::OamSprite
                    } else {
                        GraphicSource::ChrRamScene
                    };
                    let id = GraphicId::new(source, sc_idx, t_idx as u16);
                    let alt_id = GraphicId::new(
                        if is_active_oam {
                            GraphicSource::ChrRamScene
                        } else {
                            GraphicSource::OamSprite
                        },
                        sc_idx,
                        t_idx as u16,
                    );

                    let (original_bytes, current_bytes) = if let Some(patch) = self
                        .tile_patches
                        .get(&id)
                        .or_else(|| self.tile_patches.get(&alt_id))
                    {
                        (patch.original_bytes, patch.modified_bytes)
                    } else {
                        (orig_b, orig_b)
                    };

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
                        id,
                        original_bytes,
                        bytes: current_bytes,
                        label,
                        category,
                        scene_idx: sc_idx,
                        tile_in_scene: t_idx,
                        prg_offset: None,
                        is_active_in_oam: is_active_oam,
                    });
                }
            }
        }

        // 2. Gráficos de bancos CHR-ROM reales (cuando el cartucho tiene CHR-ROM dedicado)
        if !self.is_chr_ram && !self.chr_mem.is_empty() {
            let chr_source_buf = if !self.chr_rom_original.is_empty() {
                &self.chr_rom_original
            } else {
                &self.chr_mem
            };
            let total = chr_source_buf.len() / 16;
            for t_idx in 0..total {
                let base = t_idx * 16;
                let mut orig_b = [0u8; 16];
                orig_b.copy_from_slice(&chr_source_buf[base..base + 16]);
                let bank_8kb = t_idx / 512;
                let tile_in_bank = (t_idx % 512) as u16;
                let id = GraphicId::new(GraphicSource::ChrRom, bank_8kb, tile_in_bank);
                let (original_bytes, current_bytes) =
                    if let Some(patch) = self.tile_patches.get(&id) {
                        (patch.original_bytes, patch.modified_bytes)
                    } else {
                        (orig_b, orig_b)
                    };

                self.global_tiles.push(GlobalTile {
                    id,
                    original_bytes,
                    bytes: current_bytes,
                    label: format!("Banco CHR #{} — Tile #{:03X}", bank_8kb, tile_in_bank),
                    category: format!("Banco CHR #{} (8 KB)", bank_8kb),
                    scene_idx: 0,
                    tile_in_scene: t_idx,
                    prg_offset: None,
                    is_active_in_oam: false,
                });
            }
        }

        // 3. Fase G2: Candidatos heurísticos de PRG-ROM (filtrados contra opcodes 6502 y ruido)
        // Se almacenan en `self.prg_candidate_tiles` y NUNCA contaminan `global_tiles` a menos que
        // el usuario active explícitamente `show_advanced_prg_candidates`.
        let prg_source_buf = if !self.prg_rom_original.is_empty() {
            &self.prg_rom_original
        } else {
            &self.prg_rom
        };
        let total_prg_banks = prg_source_buf.len() / 16384;
        for b_idx in 0..total_prg_banks {
            let bank_name = format!("🔧 Candidatos PRG Banco #{}", b_idx);
            let bank_start = b_idx * 16384;
            let bank_end = bank_start + 16384;
            let bank_slice = &prg_source_buf[bank_start..bank_end];
            for t_in_b in 0..(16384 / 16) {
                let off = t_in_b * 16;
                let mut orig_b = [0u8; 16];
                orig_b.copy_from_slice(&bank_slice[off..off + 16]);
                if AutoMemoryMapper::is_likely_6502_code_or_noise(&orig_b) {
                    continue;
                }
                let id = GraphicId::new(GraphicSource::PrgCandidate, b_idx, t_in_b as u16);
                let (original_bytes, current_bytes) =
                    if let Some(patch) = self.tile_patches.get(&id) {
                        (patch.original_bytes, patch.modified_bytes)
                    } else {
                        (orig_b, orig_b)
                    };
                let candidate = GlobalTile {
                    id,
                    original_bytes,
                    bytes: current_bytes,
                    label: format!("Candidato PRG B#{} — #{:03X}", b_idx, t_in_b),
                    category: bank_name.clone(),
                    scene_idx: 0,
                    tile_in_scene: t_in_b,
                    prg_offset: Some(bank_start + off),
                    is_active_in_oam: false,
                };
                if self.show_advanced_prg_candidates {
                    self.global_tiles.push(candidate.clone());
                }
                self.prg_candidate_tiles.push(candidate);
            }
        }
    }

    #[allow(dead_code)]
    pub fn total_tiles(&self) -> usize {
        self.global_tiles.len()
    }

    pub fn get_tile_pixels(&self, global_idx: usize) -> [[u8; 8]; 8] {
        let Some(t) = self.global_tiles.get(global_idx) else {
            return [[0u8; 8]; 8];
        };
        AutoMemoryMapper::decode_tile_pixels(&t.bytes)
    }

    pub fn set_tile_pixel(&mut self, global_idx: usize, row: usize, col: usize, color_idx: u8) {
        if global_idx >= self.global_tiles.len() || row >= 8 || col >= 8 {
            return;
        }
        let id = self.global_tiles[global_idx].id.clone();
        let orig = self.global_tiles[global_idx].original_bytes;
        let mut modified = self.global_tiles[global_idx].bytes;
        AutoMemoryMapper::encode_pixel_in_tile(&mut modified, row, col, color_idx);
        self.apply_tile_patch_by_id(id, orig, modified);
    }

    /// Aplica un parche no destructivo identificado por `GraphicId` y actualiza automáticamente
    /// el mapa de memoria correspondiente (`CHR-ROM` por banco, `CHR-RAM` en escena + firma + offsets `PRG-ROM`)
    /// sin contaminar otros bancos ni requerir intervención manual del usuario.
    pub fn apply_tile_patch_by_id(
        &mut self,
        id: GraphicId,
        original_bytes: [u8; 16],
        modified_bytes: [u8; 16],
    ) {
        let prg_search_buf = if !self.prg_rom_original.is_empty() {
            &self.prg_rom_original
        } else {
            &self.prg_rom
        };
        let auto_prg_offsets = if matches!(
            id.source,
            GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg
        ) || self.is_chr_ram
        {
            AutoMemoryMapper::find_prg_pattern_offsets(prg_search_buf, &original_bytes)
        } else if let Some(existing) = self.tile_patches.get(&id) {
            existing.auto_prg_offsets.clone()
        } else {
            Vec::new()
        };

        let patch = TilePatch {
            id: id.clone(),
            original_bytes,
            modified_bytes,
            auto_prg_offsets: auto_prg_offsets.clone(),
        };
        self.tile_patches.insert(id.clone(), patch);
        self.modified_tiles
            .insert(id.tile_index as usize, modified_bytes);

        // Actualizar únicamente las entradas de `global_tiles` que correspondan a este recurso
        for t in &mut self.global_tiles {
            let same_exact = t.id == id;
            let same_scene_vram = matches!(
                id.source,
                GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg
            ) && matches!(
                t.id.source,
                GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg
            ) && t.id.bank_or_scene == id.bank_or_scene
                && t.id.tile_index == id.tile_index;

            if same_exact || same_scene_vram {
                t.bytes = modified_bytes;
            }
        }

        // Actualizar memoria subyacente según el origen exacto del GraphicId
        match id.source {
            GraphicSource::ChrRom => {
                let global_chr_tile = id.bank_or_scene * 512 + (id.tile_index as usize);
                let byte_off = global_chr_tile * 16;
                if byte_off + 16 <= self.chr_mem.len() {
                    self.chr_mem[byte_off..byte_off + 16].copy_from_slice(&modified_bytes);
                }
                if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                    if byte_off + 16 <= cart.chr_mem.len() {
                        cart.chr_mem[byte_off..byte_off + 16].copy_from_slice(&modified_bytes);
                    }
                }
            }
            GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg => {
                if let Some(sc) = self.scenes.get_mut(id.bank_or_scene) {
                    let byte_off = (id.tile_index as usize) * 16;
                    if byte_off + 16 <= sc.chr_8kb.len() {
                        sc.chr_8kb[byte_off..byte_off + 16].copy_from_slice(&modified_bytes);
                    }
                }
                if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                    let byte_off = (id.tile_index as usize) * 16;
                    if byte_off + 16 <= cart.chr_mem.len() {
                        cart.chr_mem[byte_off..byte_off + 16].copy_from_slice(&modified_bytes);
                    }
                }
            }
            GraphicSource::PrgCandidate => {
                let byte_off = id.bank_or_scene * 16384 + (id.tile_index as usize) * 16;
                if byte_off + 16 <= self.prg_rom.len() {
                    self.prg_rom[byte_off..byte_off + 16].copy_from_slice(&modified_bytes);
                }
                if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                    if byte_off + 16 <= cart.prg_rom.len() {
                        cart.prg_rom[byte_off..byte_off + 16].copy_from_slice(&modified_bytes);
                    }
                }
            }
            GraphicSource::ExtendedBank => {}
        }

        // Si `AutoMemoryMapper` detectó el origen en PRG-ROM, sincronizarlo automáticamente
        for &prg_off in &auto_prg_offsets {
            if prg_off + 16 <= self.prg_rom.len() {
                self.prg_rom[prg_off..prg_off + 16].copy_from_slice(&modified_bytes);
            }
            if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                if prg_off + 16 <= cart.prg_rom.len() {
                    cart.prg_rom[prg_off..prg_off + 16].copy_from_slice(&modified_bytes);
                }
            }
        }
    }

    /// Restaura de forma no destructiva un recurso `GraphicId` a sus bytes originales de fábrica.
    pub fn restore_tile_original(&mut self, id: &GraphicId) -> bool {
        let Some(patch) = self.tile_patches.remove(id) else {
            return false;
        };
        let orig = patch.original_bytes;

        for t in &mut self.global_tiles {
            let same_exact = &t.id == id;
            let same_scene_vram = matches!(
                id.source,
                GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg
            ) && matches!(
                t.id.source,
                GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg
            ) && t.id.bank_or_scene == id.bank_or_scene
                && t.id.tile_index == id.tile_index;

            if same_exact || same_scene_vram {
                t.bytes = orig;
            }
        }

        match id.source {
            GraphicSource::ChrRom => {
                let global_chr_tile = id.bank_or_scene * 512 + (id.tile_index as usize);
                let byte_off = global_chr_tile * 16;
                if byte_off + 16 <= self.chr_mem.len() {
                    self.chr_mem[byte_off..byte_off + 16].copy_from_slice(&orig);
                }
                if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                    if byte_off + 16 <= cart.chr_mem.len() {
                        cart.chr_mem[byte_off..byte_off + 16].copy_from_slice(&orig);
                    }
                }
            }
            GraphicSource::ChrRamScene | GraphicSource::OamSprite | GraphicSource::NametableBg => {
                if let Some(sc) = self.scenes.get_mut(id.bank_or_scene) {
                    let byte_off = (id.tile_index as usize) * 16;
                    if byte_off + 16 <= sc.chr_8kb.len() {
                        sc.chr_8kb[byte_off..byte_off + 16].copy_from_slice(&orig);
                    }
                }
                if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                    let byte_off = (id.tile_index as usize) * 16;
                    if byte_off + 16 <= cart.chr_mem.len() {
                        cart.chr_mem[byte_off..byte_off + 16].copy_from_slice(&orig);
                    }
                }
            }
            GraphicSource::PrgCandidate => {
                let byte_off = id.bank_or_scene * 16384 + (id.tile_index as usize) * 16;
                if byte_off + 16 <= self.prg_rom.len() {
                    self.prg_rom[byte_off..byte_off + 16].copy_from_slice(&orig);
                }
                if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                    if byte_off + 16 <= cart.prg_rom.len() {
                        cart.prg_rom[byte_off..byte_off + 16].copy_from_slice(&orig);
                    }
                }
            }
            GraphicSource::ExtendedBank => {}
        }

        for prg_off in patch.auto_prg_offsets {
            if prg_off + 16 <= self.prg_rom.len() {
                self.prg_rom[prg_off..prg_off + 16].copy_from_slice(&orig);
            }
            if let Some(cart) = self.nes_live.bus.cartridge.as_mut() {
                if prg_off + 16 <= cart.prg_rom.len() {
                    cart.prg_rom[prg_off..prg_off + 16].copy_from_slice(&orig);
                }
            }
        }

        if !self
            .tile_patches
            .keys()
            .any(|k| k.tile_index == id.tile_index)
        {
            self.modified_tiles.remove(&(id.tile_index as usize));
        }

        true
    }

    /// Re-aplica todos los `tile_patches` activos sobre `chr_mem`, `prg_rom` y `nes_live` (al cargar un Workspace).
    pub fn apply_all_patches_to_memory(&mut self) {
        let patches: Vec<TilePatch> = self.tile_patches.values().cloned().collect();
        for p in patches {
            self.apply_tile_patch_by_id(p.id, p.original_bytes, p.modified_bytes);
        }
    }

    /// Intercepta en tiempo real la VRAM (`chr_mem`) de juegos `CHR-RAM`:
    /// si el juego acaba de descomprimir o copiar desde `PRG-ROM` un patrón cuya firma de 16 bytes
    /// coincide con `patch.original_bytes`, lo sustituye automáticamente por `patch.modified_bytes`.
    pub fn sync_live_chr_ram_signatures(&mut self) {
        if self.tile_patches.is_empty() {
            return;
        }
        let Some(cart) = self.nes_live.bus.cartridge.as_mut() else {
            return;
        };
        for patch in self.tile_patches.values() {
            match patch.id.source {
                GraphicSource::ChrRamScene
                | GraphicSource::OamSprite
                | GraphicSource::NametableBg => {
                    let byte_off = (patch.id.tile_index as usize) * 16;
                    if byte_off + 16 <= cart.chr_mem.len()
                        && cart.chr_mem[byte_off..byte_off + 16] == patch.original_bytes
                    {
                        cart.chr_mem[byte_off..byte_off + 16]
                            .copy_from_slice(&patch.modified_bytes);
                    }
                }
                GraphicSource::ChrRom => {
                    let global_chr_tile =
                        patch.id.bank_or_scene * 512 + (patch.id.tile_index as usize);
                    let byte_off = global_chr_tile * 16;
                    if byte_off + 16 <= cart.chr_mem.len() {
                        cart.chr_mem[byte_off..byte_off + 16]
                            .copy_from_slice(&patch.modified_bytes);
                    }
                }
                _ => {}
            }
        }
    }

    /// Si el proyecto tiene un Workspace activo en disco, sincroniza automáticamente los cambios.
    pub fn auto_sync_workspace(&self) {
        if let Some(ws_dir) = &self.workspace_dir {
            let _ = crate::workspace::save_workspace(self, ws_dir);
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
        self.rebuild_global_tile_catalog();
    }

    #[allow(dead_code)]
    pub fn save_project_manifest(&self, path: &Path) -> Result<(), String> {
        let target_dir = if path.is_dir() {
            path.to_path_buf()
        } else if let Some(ws_dir) = &self.workspace_dir {
            ws_dir.clone()
        } else {
            path.parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf()
        };
        crate::workspace::save_workspace(self, &target_dir).map(|_| ())
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
    use std::fs;

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

    #[test]
    fn test_graphic_id_separates_banks_and_sources() {
        let mut project = NgsProject::default();
        project.loaded = true;
        project.is_chr_ram = false;
        // 2 bancos CHR-ROM de 8 KB (16 KB en total = 1024 tiles)
        project.chr_mem = vec![0x11u8; 16384];
        // Hacer que el tile #10 del Banco 0 y el tile #10 del Banco 1 tengan bytes iniciales distintos
        let bank0_t10_off = 10 * 16;
        let bank1_t10_off = (512 + 10) * 16;
        let orig_b0: [u8; 16] = [
            0x18, 0x3C, 0x7E, 0xFF, 0xFF, 0x7E, 0x3C, 0x18, 0x00, 0x18, 0x3C, 0x7E, 0x7E, 0x3C,
            0x18, 0x00,
        ];
        let orig_b1: [u8; 16] = [
            0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xAA, 0x55, 0xF0, 0x0F, 0xF0, 0x0F, 0xF0, 0x0F,
            0xF0, 0x0F,
        ];
        project.chr_mem[bank0_t10_off..bank0_t10_off + 16].copy_from_slice(&orig_b0);
        project.chr_mem[bank1_t10_off..bank1_t10_off + 16].copy_from_slice(&orig_b1);

        project.rebuild_global_tile_catalog();

        let id_b0 = GraphicId::new(GraphicSource::ChrRom, 0, 10);
        let id_b1 = GraphicId::new(GraphicSource::ChrRom, 1, 10);
        assert_ne!(id_b0, id_b1);

        let idx_b0 = project
            .global_tiles
            .iter()
            .position(|t| t.id == id_b0)
            .expect("Debe existir el tile #10 del Banco 0");
        let idx_b1 = project
            .global_tiles
            .iter()
            .position(|t| t.id == id_b1)
            .expect("Debe existir el tile #10 del Banco 1");

        // Pintar un píxel en el tile #10 del Banco 0
        project.set_tile_pixel(idx_b0, 0, 0, 3);

        assert_ne!(project.global_tiles[idx_b0].bytes, orig_b0);
        assert_eq!(
            project.global_tiles[idx_b1].bytes, orig_b1,
            "Editar el tile #10 del Banco 0 NO debe alterar el tile #10 del Banco 1"
        );
        assert_eq!(
            &project.chr_mem[bank1_t10_off..bank1_t10_off + 16],
            &orig_b1
        );

        // Restaurar el tile #10 del Banco 0 a su estado original
        assert!(project.restore_tile_original(&id_b0));
        assert_eq!(project.global_tiles[idx_b0].bytes, orig_b0);
        assert_eq!(
            &project.chr_mem[bank0_t10_off..bank0_t10_off + 16],
            &orig_b0
        );
    }

    #[test]
    fn test_chr_ram_auto_memory_mapper_signature_and_prg_sync() {
        let mut project = NgsProject::default();
        project.loaded = true;
        project.is_chr_ram = true;
        // Simular una ROM CHR-RAM con 2 bancos PRG (32 KB) llenos de código 6502 (LDA/STA/JSR/RTS)
        let mut prg = vec![0u8; 32768];
        let code_seq: [u8; 16] = [
            0xA9, 0x01, 0x8D, 0x00, 0x20, 0x20, 0x00, 0x81, 0xA5, 0x10, 0xD0, 0xF8, 0x60, 0xEA,
            0x4C, 0x00,
        ];
        for chunk in prg.chunks_exact_mut(16) {
            chunk.copy_from_slice(&code_seq);
        }

        // Insertar un patrón de personaje real en PRG offset 0x1240
        let zelda_like_sprite: [u8; 16] = [
            0x18, 0x3C, 0x7E, 0x7E, 0x3C, 0x3C, 0x18, 0x18, 0x00, 0x18, 0x3C, 0x3C, 0x18, 0x00,
            0x18, 0x00,
        ];
        prg[0x1240..0x1250].copy_from_slice(&zelda_like_sprite);
        project.prg_rom = prg;

        // Simular escena capturada donde ese sprite fue copiado a VRAM tile #5 y está activo en OAM
        let mut chr_8kb = vec![0u8; 8192];
        chr_8kb[5 * 16..5 * 16 + 16].copy_from_slice(&zelda_like_sprite);
        let mut oam = [0xFFu8; 256];
        oam[0] = 100; // Y
        oam[1] = 5;   // Tile #5
        oam[2] = 0;   // Attr
        oam[3] = 120; // X

        project.scenes.push(CapturedScene {
            title: "En Juego — Sprites y Fondo Activos".to_string(),
            chr_8kb,
            palette_table: [0x0F; 32],
            oam,
            ppu_ctrl_bits: 0x90,
            screen_rgb: vec![0; 256 * 240 * 3],
            save_state: Vec::new(),
        });

        project.rebuild_global_tile_catalog();

        // Fase G2: Verificar que los 2,048 bloques de PRG-ROM NO se volcaron en `global_tiles`
        assert_eq!(
            project.global_tiles.len(),
            512,
            "En modo CHR-RAM con 1 escena, global_tiles solo debe tener los 512 tiles de la escena y no miles de bloques PRG"
        );
        assert!(
            project.global_tiles[5].is_active_in_oam,
            "El tile #5 debe detectarse como activo en OAM"
        );

        // Fase G1: Pintar un píxel sobre el tile #5 y verificar que AutoMemoryMapper localiza y actualiza PRG 0x1240 automáticamente
        project.set_tile_pixel(5, 0, 0, 2);
        let tile_id = project.global_tiles[5].id.clone();
        let patch = project
            .tile_patches
            .get(&tile_id)
            .expect("Debe haberse registrado el TilePatch para el tile #5");
        assert_eq!(
            patch.auto_prg_offsets,
            vec![0x1240],
            "AutoMemoryMapper debe localizar automáticamente el offset 0x1240 en PRG-ROM"
        );
        assert_eq!(
            &project.prg_rom[0x1240..0x1250],
            &patch.modified_bytes,
            "PRG-ROM en 0x1240 debe sincronizarse automáticamente con los bytes editados"
        );

        // Restaurar el original y comprobar que tanto VRAM como PRG-ROM en 0x1240 vuelven a su estado original
        assert!(project.restore_tile_original(&tile_id));
        assert_eq!(&project.prg_rom[0x1240..0x1250], &zelda_like_sprite);
    }

    #[test]
    fn test_workspace_new_and_existing_roundtrip() {
        let temp_root = std::env::temp_dir().join("ngs_test_workspace_g0");
        let _ = fs::remove_dir_all(&temp_root);

        let mut project = NgsProject::default();
        let create_res = crate::workspace::create_workspace_for_new_game(
            "Aventura_Heroica",
            crate::workspace::WorkspaceTemplate::TopDownAdventure,
            &temp_root,
            &mut project,
        );
        assert!(create_res.is_ok(), "Fallo al crear workspace: {:?}", create_res);
        assert!(temp_root.join("project.ngs.json").exists());
        assert!(temp_root.join("base").exists());
        assert!(temp_root.join("src").exists());
        assert!(temp_root.join("scenes").exists());

        // Editar el tile #0 del héroe y sincronizar el workspace
        let orig_hero_tile0 = project.global_tiles[0].bytes;
        project.set_tile_pixel(0, 1, 1, 3);
        let modified_hero_tile0 = project.global_tiles[0].bytes;
        assert_ne!(orig_hero_tile0, modified_hero_tile0);
        project.auto_sync_workspace();

        // Cargar el workspace en un proyecto totalmente nuevo y verificar que conserva todo
        let mut reloaded = NgsProject::default();
        let load_res =
            crate::workspace::load_workspace(&temp_root.join("project.ngs.json"), &mut reloaded);
        assert!(load_res.is_ok(), "Fallo al recargar workspace: {:?}", load_res);
        assert_eq!(reloaded.game_title, "Aventura_Heroica");
        assert_eq!(reloaded.tile_patches.len(), 1);
        assert_eq!(reloaded.global_tiles[0].bytes, modified_hero_tile0);

        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn test_oam_8x8_and_8x16_metasprite_reconstruction() {
        use crate::graphics::metasprite::reconstruct_scene_metasprites;

        // 1. Caso 8x8 con tiles NO contiguos en CHR ($02, $02 espejado con FlipH, $1A, $45)
        let mut oam = [0xFFu8; 256];
        // Top-Left (X=100, Y=80 -> render Y=81), tile $02, sin flip
        oam[0..4].copy_from_slice(&[80, 0x02, 0x00, 100]);
        // Top-Right (X=108, Y=80 -> render Y=81), mismo tile $02 pero con FlipH (0x40)
        oam[4..8].copy_from_slice(&[80, 0x02, 0x40, 108]);
        // Bottom-Left (X=100, Y=88 -> render Y=89), tile no contiguo $1A
        oam[8..12].copy_from_slice(&[88, 0x1A, 0x00, 100]);
        // Bottom-Right (X=108, Y=88 -> render Y=89), tile no contiguo $45
        oam[12..16].copy_from_slice(&[88, 0x45, 0x00, 108]);

        let custom_names = HashMap::new();
        let entities = reconstruct_scene_metasprites(&oam, 0x00, &custom_names);
        assert_eq!(
            entities.len(),
            1,
            "Los 4 sprites 8x8 adyacentes deben agruparse en 1 sola entidad MetaspriteEntity"
        );
        let hero = &entities[0];
        assert_eq!(hero.width, 16);
        assert_eq!(hero.height, 16);
        assert_eq!(hero.pieces.len(), 4);

        // Verificar mapeo inverso de píxeles:
        // En la mitad derecha espejada con FlipH (canvas_x = 8 -> local_x = 0), la columna real del tile $02 debe ser 7!
        let (vram_tile, row, col, _) = hero
            .map_canvas_pixel_to_vram_tile(8, 0)
            .expect("Debe mapear el píxel (8,0) de la mitad derecha");
        assert_eq!(vram_tile, 0x02);
        assert_eq!(row, 0);
        assert_eq!(
            col, 7,
            "Con FlipH activo en la mitad derecha, el píxel local X=0 debe mapear a la columna 7 del tile"
        );

        // 2. Caso 8x16 (bit 5 de PPUCTRL = 0x20) con FlipV (0x80)
        let mut oam_8x16 = [0xFFu8; 256];
        // Sprite 8x16 con tile_id = 0x07 (impar -> banco $1000 = +256, par superior = $06 -> 262, inferior = $07 -> 263), con FlipV (0x80)
        oam_8x16[0..4].copy_from_slice(&[50, 0x07, 0x80, 60]);
        let entities_8x16 = reconstruct_scene_metasprites(&oam_8x16, 0x20, &custom_names);
        assert_eq!(entities_8x16.len(), 1);
        let tall = &entities_8x16[0];
        assert_eq!(tall.width, 8);
        assert_eq!(tall.height, 16);
        assert_eq!(tall.pieces.len(), 2);
        // Al tener FlipV activo en modo 8x16, la pieza superior (rel_y = 0) debe usar el tile inferior (263) y la inferior (rel_y = 8) el superior (262)
        assert_eq!(tall.pieces[0].vram_tile_id, 263);
        assert_eq!(tall.pieces[1].vram_tile_id, 262);
    }

    #[test]
    fn test_png_replacement_and_project_roundtrip() {
        use crate::graphics::replacement::{
            decode_rgba_from_bytes, encode_uncompressed_png_rgba, register_extended_hd_asset,
            DecodedRgbaImage,
        };

        let temp_root = std::env::temp_dir().join("ngs_test_workspace_g5_png");
        let _ = fs::remove_dir_all(&temp_root);

        let mut project = NgsProject::default();
        crate::workspace::create_workspace_for_new_game(
            "Test_PNG_HD",
            crate::workspace::WorkspaceTemplate::PlatformerAction,
            &temp_root,
            &mut project,
        )
        .expect("Workspace creado");

        // Crear una imagen RGBA 16x16 de prueba, codificarla a un archivo .PNG real y decodificarla
        let mut pixels = Vec::with_capacity(16 * 16);
        for i in 0..(16 * 16) {
            pixels.push(DecodedRgbaImage::pack_rgba(
                (i & 0xFF) as u8,
                120,
                240,
                255,
            ));
        }
        let png_bytes = encode_uncompressed_png_rgba(16, 16, &pixels);
        let decoded = decode_rgba_from_bytes(&png_bytes).expect("Debe decodificar el PNG RGBA");
        assert_eq!(decoded.width, 16);
        assert_eq!(decoded.height, 16);
        assert_eq!(decoded.pixels, pixels);

        // Registrar en el Banco de Memoria Extendida Ilimitada y guardar en el Workspace
        let target_id = GraphicId::new(GraphicSource::OamSprite, 0, 0);
        let asset_id = register_extended_hd_asset(
            &mut project,
            "Heroe_HD_PNG".to_string(),
            Some(target_id.clone()),
            &decoded,
            Some("heroe_hd.png".to_string()),
        );
        assert_eq!(asset_id, 1);

        // Recargar el Workspace desde disco y verificar que el activo HD RGBA persiste intacto
        let mut reloaded = NgsProject::default();
        crate::workspace::load_workspace(&temp_root.join("project.ngs.json"), &mut reloaded)
            .expect("Debe recargar el workspace con el activo PNG HD");
        assert_eq!(reloaded.extended_assets.len(), 1);
        assert_eq!(reloaded.extended_assets[0].name, "Heroe_HD_PNG");
        assert_eq!(reloaded.extended_assets[0].width, 16);
        assert_eq!(reloaded.extended_assets[0].height, 16);
        assert_eq!(reloaded.extended_assets[0].rgba_pixels, pixels);
        assert_eq!(
            reloaded.extended_assets[0].target_graphic_id,
            Some(target_id)
        );

        let _ = fs::remove_dir_all(&temp_root);
    }
}
