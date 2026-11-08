use crate::graphics::oam_inspector::{inspect_scene_oam, OamEntry};
use crate::graphics::AutoMemoryMapper;
use crate::project::nes_color_rgb_u8;
use std::collections::HashMap;

/// Sub-pieza de `8x8` píxeles dentro de un personaje o entidad reconstruida desde `OAM` (Fase G3).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetaspritePiece {
    pub oam_index: u8,
    /// Desplazamiento X relativo a la esquina superior izquierda (`bbox_x`) de la entidad.
    pub rel_x: u16,
    /// Desplazamiento Y relativo a la esquina superior izquierda (`bbox_y`) de la entidad.
    pub rel_y: u16,
    /// Índice del tile `8x8` dentro de los 8 KB de VRAM (`0..511`).
    pub vram_tile_id: u16,
    /// Paleta de sprite (`0..3`).
    pub palette_idx: u8,
    pub flip_h: bool,
    pub flip_v: bool,
    pub behind_bg: bool,
}

/// Entidad visual completa (personaje, enemigo, jefe, proyectil u objeto interactivo)
/// reconstruida automáticamente agrupando sprites `OAM` adyacentes en pantalla (Fase G3).
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct MetaspriteEntity {
    pub entity_id: usize,
    /// Clave estable basada en los tiles constitutivos para asociar nombres personalizados o gráficos HD.
    pub signature_key: String,
    pub name: String,
    pub bbox_x: u8,
    pub bbox_y: u8,
    pub width: u16,
    pub height: u16,
    pub primary_palette: u8,
    pub is_8x16_mode: bool,
    pub pieces: Vec<MetaspritePiece>,
}

impl MetaspriteEntity {
    /// Verifica si el punto `(screen_x, screen_y)` en coordenadas NES (`256x240`) cae dentro de esta entidad.
    pub fn contains_screen_point(&self, screen_x: u16, screen_y: u16) -> bool {
        let x0 = self.bbox_x as u16;
        let y0 = self.bbox_y as u16;
        screen_x >= x0
            && screen_x < x0.saturating_add(self.width)
            && screen_y >= y0
            && screen_y < y0.saturating_add(self.height)
    }

    /// Devuelve la lista ordenada y sin duplicados de índices de tiles VRAM (`0..511`) que componen la entidad.
    pub fn unique_vram_tiles(&self) -> Vec<u16> {
        let mut out = Vec::new();
        for p in &self.pieces {
            if !out.contains(&p.vram_tile_id) {
                out.push(p.vram_tile_id);
            }
        }
        out
    }

    /// Mapea una coordenada `(canvas_x, canvas_y)` del lienzo unificado del personaje
    /// hacia el tile VRAM exacto y su fila/columna interna `(vram_tile_id, row, col, palette_idx)`,
    /// invirtiendo automáticamente `FlipH` y `FlipV` para que el usuario pinte de forma natural.
    pub fn map_canvas_pixel_to_vram_tile(
        &self,
        canvas_x: u16,
        canvas_y: u16,
    ) -> Option<(u16, usize, usize, u8)> {
        // Recorrer en orden de prioridad OAM (menor índice OAM tiene prioridad visual en la NES)
        for piece in &self.pieces {
            if canvas_x >= piece.rel_x
                && canvas_x < piece.rel_x + 8
                && canvas_y >= piece.rel_y
                && canvas_y < piece.rel_y + 8
            {
                let local_x = (canvas_x - piece.rel_x) as usize;
                let local_y = (canvas_y - piece.rel_y) as usize;
                let col = if piece.flip_h { 7 - local_x } else { local_x };
                let row = if piece.flip_v { 7 - local_y } else { local_y };
                return Some((piece.vram_tile_id, row, col, piece.palette_idx));
            }
        }
        None
    }

    /// Renderiza el personaje completo en un buffer RGBA (`width * height` píxeles `[R, G, B, A]`)
    /// aplicando paletas de sprite reales, transparencia en color `0` y espejado `FlipH`/`FlipV`.
    pub fn render_rgba_canvas(
        &self,
        chr_8kb: &[u8],
        palette_table: &[u8; 32],
    ) -> Vec<[u8; 4]> {
        let w = (self.width as usize).max(8);
        let h = (self.height as usize).max(8);
        let mut rgba = vec![[0u8, 0u8, 0u8, 0u8]; w * h];

        // En la NES, los índices OAM más bajos se dibujan ENCIMA de los índices más altos,
        // por lo que pintamos de atrás hacia adelante (`rev()`).
        for piece in self.pieces.iter().rev() {
            let byte_off = (piece.vram_tile_id as usize) * 16;
            if byte_off + 16 > chr_8kb.len() {
                continue;
            }
            let mut tile_bytes = [0u8; 16];
            tile_bytes.copy_from_slice(&chr_8kb[byte_off..byte_off + 16]);
            let decoded = AutoMemoryMapper::decode_tile_pixels(&tile_bytes);
            let pal_base = 16 + ((piece.palette_idx as usize) & 3) * 4;

            for ly in 0..8usize {
                for lx in 0..8usize {
                    let src_row = if piece.flip_v { 7 - ly } else { ly };
                    let src_col = if piece.flip_h { 7 - lx } else { lx };
                    let color_idx = decoded[src_row][src_col];
                    if color_idx == 0 {
                        continue; // Transparente en sprites NES
                    }
                    let nes_color = palette_table[(pal_base + (color_idx as usize)) & 31];
                    let [r, g, b] = nes_color_rgb_u8(nes_color);
                    let dst_x = (piece.rel_x as usize) + lx;
                    let dst_y = (piece.rel_y as usize) + ly;
                    if dst_x < w && dst_y < h {
                        rgba[dst_y * w + dst_x] = [r, g, b, 255];
                    }
                }
            }
        }

        rgba
    }
}

/// Reconstruye todas las entidades visibles (`MetaspriteEntity`) en una escena a partir de su `OAM` y `PPUCTRL`.
/// Agrupa automáticamente por proximidad espacial (`touching / near bounding boxes`) aunque los tiles en `CHR`
/// no sean contiguos ni tengan dimensiones fijas `16x32`.
pub fn reconstruct_scene_metasprites(
    oam: &[u8; 256],
    ppu_ctrl_bits: u8,
    custom_names: &HashMap<String, String>,
) -> Vec<MetaspriteEntity> {
    let entries = inspect_scene_oam(oam, ppu_ctrl_bits);
    if entries.is_empty() {
        return Vec::new();
    }

    let n = entries.len();
    let mut visited = vec![false; n];
    let mut entities = Vec::new();

    for start_idx in 0..n {
        if visited[start_idx] {
            continue;
        }
        visited[start_idx] = true;
        let mut cluster_indices = vec![start_idx];
        let mut queue = vec![start_idx];

        while let Some(curr) = queue.pop() {
            let a = &entries[curr];
            for cand_idx in 0..n {
                if visited[cand_idx] {
                    continue;
                }
                let b = &entries[cand_idx];
                if entries_belong_to_same_entity(a, b, cluster_indices.len()) {
                    visited[cand_idx] = true;
                    cluster_indices.push(cand_idx);
                    queue.push(cand_idx);
                }
            }
        }

        cluster_indices.sort_by_key(|&i| entries[i].oam_index);

        let min_x = cluster_indices
            .iter()
            .map(|&i| entries[i].x)
            .min()
            .unwrap_or(0);
        let min_y = cluster_indices
            .iter()
            .map(|&i| entries[i].y)
            .min()
            .unwrap_or(0);
        let max_x = cluster_indices
            .iter()
            .map(|&i| (entries[i].x as u16) + entries[i].width())
            .max()
            .unwrap_or(8);
        let max_y = cluster_indices
            .iter()
            .map(|&i| (entries[i].y as u16) + entries[i].height())
            .max()
            .unwrap_or(8);

        let width = max_x.saturating_sub(min_x as u16).max(8);
        let height = max_y.saturating_sub(min_y as u16).max(8);
        let primary_palette = entries[cluster_indices[0]].palette_idx;
        let is_8x16_mode = entries[cluster_indices[0]].is_8x16;

        let mut pieces = Vec::new();
        for &idx in &cluster_indices {
            let e = &entries[idx];
            let base_rel_x = (e.x as u16).saturating_sub(min_x as u16);
            let base_rel_y = (e.y as u16).saturating_sub(min_y as u16);

            if let Some(bottom_tile) = e.bottom_vram_tile {
                // En modo 8x16, si flip_v está activo, el hardware NES invierte el orden vertical
                // entre el tile superior y el tile inferior.
                let (first_tile, second_tile) = if e.flip_v {
                    (bottom_tile, e.top_vram_tile)
                } else {
                    (e.top_vram_tile, bottom_tile)
                };
                pieces.push(MetaspritePiece {
                    oam_index: e.oam_index,
                    rel_x: base_rel_x,
                    rel_y: base_rel_y,
                    vram_tile_id: first_tile,
                    palette_idx: e.palette_idx,
                    flip_h: e.flip_h,
                    flip_v: e.flip_v,
                    behind_bg: e.behind_bg,
                });
                pieces.push(MetaspritePiece {
                    oam_index: e.oam_index,
                    rel_x: base_rel_x,
                    rel_y: base_rel_y + 8,
                    vram_tile_id: second_tile,
                    palette_idx: e.palette_idx,
                    flip_h: e.flip_h,
                    flip_v: e.flip_v,
                    behind_bg: e.behind_bg,
                });
            } else {
                pieces.push(MetaspritePiece {
                    oam_index: e.oam_index,
                    rel_x: base_rel_x,
                    rel_y: base_rel_y,
                    vram_tile_id: e.top_vram_tile,
                    palette_idx: e.palette_idx,
                    flip_h: e.flip_h,
                    flip_v: e.flip_v,
                    behind_bg: e.behind_bg,
                });
            }
        }

        // Construir firma estable a partir del primer tile y dimensiones
        let first_tile = pieces.first().map(|p| p.vram_tile_id).unwrap_or(0);
        let signature_key = format!("ENT_T{:03X}_{}x{}", first_tile, width, height);
        let entity_id = entities.len() + 1;
        let default_name = if entity_id == 1 {
            format!("Personaje #1 ({}x{})", width, height)
        } else {
            format!("Entidad #{} ({}x{})", entity_id, width, height)
        };
        let name = custom_names
            .get(&signature_key)
            .cloned()
            .unwrap_or(default_name);

        entities.push(MetaspriteEntity {
            entity_id,
            signature_key,
            name,
            bbox_x: min_x,
            bbox_y: min_y,
            width,
            height,
            primary_palette,
            is_8x16_mode,
            pieces,
        });
    }

    entities
}

/// Determina si dos entradas OAM pertenecen al mismo personaje/objeto compuesto en pantalla.
fn entries_belong_to_same_entity(a: &OamEntry, b: &OamEntry, current_cluster_size: usize) -> bool {
    if current_cluster_size >= 16 {
        return false; // Evitar fusionar toda la pantalla si hay muchos sprites pegados
    }
    let ax0 = a.x as i32;
    let ay0 = a.y as i32;
    let ax1 = ax0 + (a.width() as i32);
    let ay1 = ay0 + (a.height() as i32);

    let bx0 = b.x as i32;
    let by0 = b.y as i32;
    let bx1 = bx0 + (b.width() as i32);
    let by1 = by0 + (b.height() as i32);

    // Distancia entre bordes de las cajas (<= 2 píxeles de separación cuenta como adyacente/conectado)
    let gap_x = (bx0 - ax1).max(ax0 - bx1);
    let gap_y = (by0 - ay1).max(ay0 - by1);

    gap_x <= 2 && gap_y <= 2
}
