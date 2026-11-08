pub mod animation;
pub mod metasprite;
pub mod oam_inspector;
pub mod replacement;

use std::fmt;

/// Origen real de un recurso gráfico dentro de NesGameStudio (Fase G1 + Fase G2).
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum GraphicSource {
    /// Banco físico de CHR-ROM (en juegos con CHR-ROM dedicado).
    ChrRom,
    /// Memoria VRAM de 8 KB capturada en una escena (juegos CHR-RAM o estado puntual).
    ChrRamScene,
    /// Tile referenciado activamente por un sprite en la OAM de una escena.
    OamSprite,
    /// Tile referenciado activamente por la Nametable (escenario/fondo) de una escena.
    NametableBg,
    /// Banco de Memoria Extendida Ilimitada (nuevas creaciones y reemplazos HD sin límite NES).
    ExtendedBank,
    /// Candidato heurístico extraído de PRG-ROM (oculto bajo la vista Avanzada).
    PrgCandidate,
}

impl GraphicSource {
    pub fn code(&self) -> &'static str {
        match self {
            GraphicSource::ChrRom => "ChrRom",
            GraphicSource::ChrRamScene => "ChrRamScene",
            GraphicSource::OamSprite => "OamSprite",
            GraphicSource::NametableBg => "NametableBg",
            GraphicSource::ExtendedBank => "ExtendedBank",
            GraphicSource::PrgCandidate => "PrgCandidate",
        }
    }

    pub fn from_code(s: &str) -> Option<Self> {
        match s.trim() {
            "ChrRom" => Some(GraphicSource::ChrRom),
            "ChrRamScene" => Some(GraphicSource::ChrRamScene),
            "OamSprite" => Some(GraphicSource::OamSprite),
            "NametableBg" => Some(GraphicSource::NametableBg),
            "ExtendedBank" => Some(GraphicSource::ExtendedBank),
            "PrgCandidate" => Some(GraphicSource::PrgCandidate),
            _ => None,
        }
    }

    pub fn label_es(&self) -> &'static str {
        match self {
            GraphicSource::ChrRom => "Banco CHR-ROM",
            GraphicSource::ChrRamScene => "Escena VRAM (CHR-RAM)",
            GraphicSource::OamSprite => "Personaje / Objeto OAM",
            GraphicSource::NametableBg => "Bloque de Escenario",
            GraphicSource::ExtendedBank => "Memoria Extendida HD",
            GraphicSource::PrgCandidate => "Candidato PRG (Avanzado)",
        }
    }
}

/// Identificador unívoco de un recurso gráfico (Fase G1).
/// Evita colisiones entre bancos CHR-ROM, escenas CHR-RAM y memoria extendida.
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct GraphicId {
    pub source: GraphicSource,
    pub bank_or_scene: usize,
    pub tile_index: u16,
}

impl GraphicId {
    pub fn new(source: GraphicSource, bank_or_scene: usize, tile_index: u16) -> Self {
        Self {
            source,
            bank_or_scene,
            tile_index,
        }
    }

    pub fn short_key(&self) -> String {
        format!(
            "{}:{}:{}",
            self.source.code(),
            self.bank_or_scene,
            self.tile_index
        )
    }

    pub fn from_short_key(key: &str) -> Option<Self> {
        let mut parts = key.split(':');
        let source = GraphicSource::from_code(parts.next()?)?;
        let bank_or_scene = parts.next()?.parse::<usize>().ok()?;
        let tile_index = parts.next()?.parse::<u16>().ok()?;
        Some(Self {
            source,
            bank_or_scene,
            tile_index,
        })
    }
}

impl fmt::Display for GraphicId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} #{} :: Tile ${:03X}",
            self.source.label_es(),
            self.bank_or_scene,
            self.tile_index
        )
    }
}

/// Parche no destructivo para un tile de 8x8 (16 bytes 2bpp) con rastreo automático en memoria (Fase G1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TilePatch {
    pub id: GraphicId,
    pub original_bytes: [u8; 16],
    pub modified_bytes: [u8; 16],
    /// Offsets dentro de PRG-ROM detectados automáticamente por `AutoMemoryMapper`
    /// cuando el juego utiliza CHR-RAM y copia patrones desde PRG-ROM.
    pub auto_prg_offsets: Vec<usize>,
}

impl TilePatch {
    #[allow(dead_code)]
    pub fn is_modified(&self) -> bool {
        self.original_bytes != self.modified_bytes
    }
}

/// Recurso almacenado en el Banco de Memoria Extendida Ilimitada (sin límites de 8 KB ni de 4 colores).
#[derive(Clone, Debug)]
pub struct ExtendedGraphicAsset {
    pub asset_id: usize,
    pub name: String,
    pub target_graphic_id: Option<GraphicId>,
    pub width: u32,
    pub height: u32,
    pub rgba_pixels: Vec<u32>,
    pub source_path: Option<String>,
}

/// Motor de Mapeo Automático Transparente de Memoria (`AutoMemoryMapper`).
/// Se encarga de localizar de dónde provienen los gráficos en `PRG-ROM` / `CHR-RAM` / `CHR-ROM`
/// y de filtrar código 6502 para que el usuario nunca tenga que preocuparse por bancos ni direcciones.
pub struct AutoMemoryMapper;

impl AutoMemoryMapper {
    /// Busca automáticamente en `prg_rom` todas las apariciones exactas de un patrón 2bpp de 16 bytes.
    /// Ignora bloques triviales (todo `0x00`, todo `0xFF` o un único byte repetido 16 veces)
    /// para no parchear espacio vacío o relleno de la ROM.
    pub fn find_prg_pattern_offsets(prg_rom: &[u8], pattern_16: &[u8; 16]) -> Vec<usize> {
        if Self::is_trivial_pattern(pattern_16) || prg_rom.len() < 16 {
            return Vec::new();
        }
        let mut offsets = Vec::new();
        let limit = prg_rom.len() - 16;
        let mut i = 0;
        while i <= limit {
            if &prg_rom[i..i + 16] == pattern_16 {
                offsets.push(i);
                if offsets.len() >= 16 {
                    break;
                }
                i += 16;
            } else {
                i += 1;
            }
        }
        offsets
    }

    /// Determina si un bloque de 16 bytes es trivial (vacío o relleno uniforme).
    pub fn is_trivial_pattern(bytes: &[u8; 16]) -> bool {
        let first = bytes[0];
        if bytes.iter().all(|&b| b == first) {
            return true;
        }
        let non_zero = bytes.iter().filter(|&&b| b != 0 && b != 0xFF).count();
        non_zero < 2
    }

    /// Heurística de Fase G2 para distinguir un patrón gráfico 2bpp real de código máquina 6502 o tablas de punteros.
    /// Retorna `true` si el bloque parece código 6502 o ruido no gráfico.
    pub fn is_likely_6502_code_or_noise(chunk: &[u8; 16]) -> bool {
        if Self::is_trivial_pattern(chunk) {
            return true;
        }

        // 1. Contar opcodes 6502 sumamente frecuentes en código compilado/ensamblado
        let mut opcode_hits = 0usize;
        for &b in chunk.iter() {
            if matches!(
                b,
                0x20 // JSR abs
                | 0x60 // RTS
                | 0x4C // JMP abs
                | 0xA9 // LDA #imm
                | 0xA5 // LDA zp
                | 0xAD // LDA abs
                | 0xBD // LDA abs,X
                | 0xB9 // LDA abs,Y
                | 0x85 // STA zp
                | 0x8D // STA abs
                | 0x9D // STA abs,X
                | 0x99 // STA abs,Y
                | 0xE8 // INX
                | 0xC8 // INY
                | 0xCA // DEX
                | 0x88 // DEY
                | 0xD0 // BNE
                | 0xF0 // BEQ
                | 0x90 // BCC
                | 0xB0 // BCS
                | 0x18 // CLC
                | 0x38 // SEC
                | 0x48 // PHA
                | 0x68 // PLA
            ) {
                opcode_hits += 1;
            }
        }
        if opcode_hits >= 5 {
            return true;
        }

        // 2. En un tile NES 2bpp real, las filas adyacentes dentro de cada bitplane (0..7 y 8..15)
        // suelen tener correlación vertical, o los dos bitplanes comparten bordes/silueta.
        let mut vertical_similarity = 0usize;
        for r in 0..7 {
            if chunk[r] == chunk[r + 1] || (chunk[r] & chunk[r + 1]) != 0 {
                vertical_similarity += 1;
            }
            if chunk[r + 8] == chunk[r + 9] || (chunk[r + 8] & chunk[r + 9]) != 0 {
                vertical_similarity += 1;
            }
        }

        let mut plane_correlation = 0usize;
        for r in 0..8 {
            let p0 = chunk[r];
            let p1 = chunk[r + 8];
            if (p0 & p1) != 0 || (p0 | p1) == p0 || (p0 | p1) == p1 {
                plane_correlation += 1;
            }
        }

        vertical_similarity < 5 || plane_correlation < 3
    }

    /// Extrae el conjunto de índices de tiles (0..511 en la ventana de 8 KB de VRAM)
    /// que están siendo utilizados activamente por los 64 sprites de la OAM en la escena dada.
    pub fn extract_active_oam_tile_indices(oam: &[u8; 256], ppu_ctrl_bits: u8) -> Vec<u16> {
        let is_8x16 = (ppu_ctrl_bits & 0x20) != 0;
        let sprite_table_base: u16 = if (ppu_ctrl_bits & 0x08) != 0 { 256 } else { 0 };
        let mut active = Vec::new();

        for spr_idx in 0..64 {
            let base = spr_idx * 4;
            let y = oam[base];
            let tile_byte = oam[base + 1];
            let x = oam[base + 3];

            // Descartar sprites ocultos fuera de la pantalla visible
            if y >= 0xEF || (x == 0 && y == 0 && tile_byte == 0) {
                continue;
            }

            if is_8x16 {
                let bank_offset: u16 = if (tile_byte & 1) != 0 { 256 } else { 0 };
                let top_tile = bank_offset + ((tile_byte & 0xFE) as u16);
                let bottom_tile = top_tile + 1;
                if !active.contains(&top_tile) {
                    active.push(top_tile);
                }
                if !active.contains(&bottom_tile) {
                    active.push(bottom_tile);
                }
            } else {
                let full_idx = sprite_table_base + (tile_byte as u16);
                if !active.contains(&full_idx) {
                    active.push(full_idx);
                }
            }
        }
        active
    }

    /// Codifica un par de bytes `(p0, p1)` al modificar un píxel `(row, col)` con `color_idx` (0..3).
    pub fn encode_pixel_in_tile(bytes: &mut [u8; 16], row: usize, col: usize, color_idx: u8) {
        if row >= 8 || col >= 8 {
            return;
        }
        let bit = 7 - col;
        let lo = color_idx & 1;
        let hi = (color_idx >> 1) & 1;
        let mut p0 = bytes[row] & !(1 << bit);
        let mut p1 = bytes[row + 8] & !(1 << bit);
        p0 |= lo << bit;
        p1 |= hi << bit;
        bytes[row] = p0;
        bytes[row + 8] = p1;
    }

    /// Decodifica un tile 2bpp de 16 bytes en una matriz de 8x8 índices de color (0..3).
    pub fn decode_tile_pixels(bytes: &[u8; 16]) -> [[u8; 8]; 8] {
        let mut out = [[0u8; 8]; 8];
        for row in 0..8 {
            let p0 = bytes[row];
            let p1 = bytes[row + 8];
            for col in 0..8 {
                let bit = 7 - col;
                let lo = (p0 >> bit) & 1;
                let hi = (p1 >> bit) & 1;
                out[row][col] = (hi << 1) | lo;
            }
        }
        out
    }
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push_str(&format!("{:02X}", b));
    }
    s
}

pub fn hex_to_bytes_16(hex: &str) -> Option<[u8; 16]> {
    let clean = hex.trim();
    if clean.len() != 32 {
        return None;
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        out[i] = u8::from_str_radix(&clean[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
