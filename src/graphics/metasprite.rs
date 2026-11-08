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
            let is_bg_palette = (piece.palette_idx & 0x80) != 0;
            let pal_num = (piece.palette_idx & 0x03) as usize;
            let pal_base = if is_bg_palette {
                pal_num * 4
            } else {
                16 + pal_num * 4
            };

            for ly in 0..8usize {
                for lx in 0..8usize {
                    let src_row = if piece.flip_v { 7 - ly } else { ly };
                    let src_col = if piece.flip_h { 7 - lx } else { lx };
                    let color_idx = decoded[src_row][src_col];
                    if color_idx == 0 && !is_bg_palette {
                        continue; // Transparente en sprites OAM
                    }
                    let nes_color = if color_idx == 0 {
                        palette_table[0]
                    } else {
                        palette_table[(pal_base + (color_idx as usize)) & 31]
                    };
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

/// Devuelve cada sprite OAM visible en pantalla como una entidad individual seleccionable (`8x8` o `8x16`),
/// para que el usuario pueda seleccionar cualquier sprite suelto sin agrupar (Fase G7).
pub fn reconstruct_individual_oam_sprites(
    oam: &[u8; 256],
    ppu_ctrl_bits: u8,
    custom_names: &HashMap<String, String>,
) -> Vec<MetaspriteEntity> {
    let entries = inspect_scene_oam(oam, ppu_ctrl_bits);
    let mut out = Vec::new();

    for (i, e) in entries.into_iter().enumerate() {
        let mut pieces = Vec::new();
        if let Some(bottom_tile) = e.bottom_vram_tile {
            let (first_tile, second_tile) = if e.flip_v {
                (bottom_tile, e.top_vram_tile)
            } else {
                (e.top_vram_tile, bottom_tile)
            };
            pieces.push(MetaspritePiece {
                oam_index: e.oam_index,
                rel_x: 0,
                rel_y: 0,
                vram_tile_id: first_tile,
                palette_idx: e.palette_idx,
                flip_h: e.flip_h,
                flip_v: e.flip_v,
                behind_bg: e.behind_bg,
            });
            pieces.push(MetaspritePiece {
                oam_index: e.oam_index,
                rel_x: 0,
                rel_y: 8,
                vram_tile_id: second_tile,
                palette_idx: e.palette_idx,
                flip_h: e.flip_h,
                flip_v: e.flip_v,
                behind_bg: e.behind_bg,
            });
        } else {
            pieces.push(MetaspritePiece {
                oam_index: e.oam_index,
                rel_x: 0,
                rel_y: 0,
                vram_tile_id: e.top_vram_tile,
                palette_idx: e.palette_idx,
                flip_h: e.flip_h,
                flip_v: e.flip_v,
                behind_bg: e.behind_bg,
            });
        }

        let w = e.width();
        let h = e.height();
        let sig = format!("OAM_SPR_{:02}_T{:03X}", e.oam_index, e.top_vram_tile);
        let default_name = format!(
            "Sprite OAM #{:02} (Tile ${:02X}, {}x{})",
            e.oam_index, e.tile_id, w, h
        );
        let name = custom_names.get(&sig).cloned().unwrap_or(default_name);

        out.push(MetaspriteEntity {
            entity_id: i + 1,
            signature_key: sig,
            name,
            bbox_x: e.x,
            bbox_y: e.y,
            width: w,
            height: h,
            primary_palette: e.palette_idx,
            is_8x16_mode: e.is_8x16,
            pieces,
        });
    }

    out
}

/// Ensambla TODOS los sprites OAM activos de la pantalla actual en una sola entidad compuesta editable (Fase G7).
pub fn reconstruct_all_oam_sprites_as_one(
    oam: &[u8; 256],
    ppu_ctrl_bits: u8,
) -> Option<MetaspriteEntity> {
    let entries = inspect_scene_oam(oam, ppu_ctrl_bits);
    if entries.is_empty() {
        return None;
    }

    let min_x = entries.iter().map(|e| e.x).min().unwrap_or(0);
    let min_y = entries.iter().map(|e| e.y).min().unwrap_or(0);
    let max_x = entries
        .iter()
        .map(|e| (e.x as u16) + e.width())
        .max()
        .unwrap_or(8);
    let max_y = entries
        .iter()
        .map(|e| (e.y as u16) + e.height())
        .max()
        .unwrap_or(8);

    let width = max_x.saturating_sub(min_x as u16).max(8);
    let height = max_y.saturating_sub(min_y as u16).max(8);
    let primary_palette = entries[0].palette_idx;
    let is_8x16_mode = entries[0].is_8x16;

    let mut pieces = Vec::new();
    for e in &entries {
        let rx = (e.x as u16).saturating_sub(min_x as u16);
        let ry = (e.y as u16).saturating_sub(min_y as u16);
        if let Some(bottom_tile) = e.bottom_vram_tile {
            let (first_tile, second_tile) = if e.flip_v {
                (bottom_tile, e.top_vram_tile)
            } else {
                (e.top_vram_tile, bottom_tile)
            };
            pieces.push(MetaspritePiece {
                oam_index: e.oam_index,
                rel_x: rx,
                rel_y: ry,
                vram_tile_id: first_tile,
                palette_idx: e.palette_idx,
                flip_h: e.flip_h,
                flip_v: e.flip_v,
                behind_bg: e.behind_bg,
            });
            pieces.push(MetaspritePiece {
                oam_index: e.oam_index,
                rel_x: rx,
                rel_y: ry + 8,
                vram_tile_id: second_tile,
                palette_idx: e.palette_idx,
                flip_h: e.flip_h,
                flip_v: e.flip_v,
                behind_bg: e.behind_bg,
            });
        } else {
            pieces.push(MetaspritePiece {
                oam_index: e.oam_index,
                rel_x: rx,
                rel_y: ry,
                vram_tile_id: e.top_vram_tile,
                palette_idx: e.palette_idx,
                flip_h: e.flip_h,
                flip_v: e.flip_v,
                behind_bg: e.behind_bg,
            });
        }
    }

    Some(MetaspriteEntity {
        entity_id: 999,
        signature_key: format!("ALL_OAM_{}x{}", width, height),
        name: format!("Todos los Sprites en Pantalla ({} piezas)", pieces.len()),
        bbox_x: min_x,
        bbox_y: min_y,
        width,
        height,
        primary_palette,
        is_8x16_mode,
        pieces,
    })
}

/// Extrae como tarjetas visuales (`16x16`) TODOS los gráficos de sprites y bloques no vacíos
/// que están cargados en los 8 KB de VRAM de la escena actual (`chr_8kb`), para que el usuario
/// pueda ver, seleccionar y editar enemigos, balas, explosiones u objetos aunque no estén en `OAM` en ese frame.
pub fn extract_all_loaded_vram_sprite_cards(
    chr_8kb: &[u8],
    ppu_ctrl_bits: u8,
    custom_names: &HashMap<String, String>,
) -> Vec<MetaspriteEntity> {
    let is_8x16 = (ppu_ctrl_bits & 0x20) != 0;
    let total_tiles = (chr_8kb.len() / 16).min(512);
    let mut cards = Vec::new();

    // Agrupar de 4 en 4 tiles (16x16 px) que contengan gráficos reales no vacíos
    let mut t_idx = 0usize;
    while t_idx + 4 <= total_tiles {
        let mut non_empty_count = 0usize;
        for k in 0..4 {
            let off = (t_idx + k) * 16;
            let mut b = [0u8; 16];
            b.copy_from_slice(&chr_8kb[off..off + 16]);
            if !AutoMemoryMapper::is_trivial_pattern(&b) {
                non_empty_count += 1;
            }
        }

        if non_empty_count > 0 {
            let mut pieces = Vec::with_capacity(4);
            // Si el juego usa modo 8x16 (como Contra), los pares (0,1) son la columna izquierda y (2,3) la derecha
            let layout: [(u16, u16, u16); 4] = if is_8x16 {
                [
                    (0, 0, t_idx as u16),
                    (0, 8, (t_idx + 1) as u16),
                    (8, 0, (t_idx + 2) as u16),
                    (8, 8, (t_idx + 3) as u16),
                ]
            } else {
                [
                    (0, 0, t_idx as u16),
                    (8, 0, (t_idx + 1) as u16),
                    (0, 8, (t_idx + 2) as u16),
                    (8, 8, (t_idx + 3) as u16),
                ]
            };

            for (rx, ry, tid) in layout {
                pieces.push(MetaspritePiece {
                    oam_index: 0,
                    rel_x: rx,
                    rel_y: ry,
                    vram_tile_id: tid,
                    palette_idx: if t_idx < 256 { 0 } else { 0x80 }, // 0x80 = paleta de escenario
                    flip_h: false,
                    flip_v: false,
                    behind_bg: t_idx >= 256,
                });
            }

            let sig = format!("VRAM_CARD_{:03X}", t_idx);
            let kind_str = if t_idx < 256 { "Grupo Sprite" } else { "Grupo Escenario" };
            let default_name = format!(
                "{} VRAM #{} (Tiles ${:03X}..${:03X})",
                kind_str,
                cards.len() + 1,
                t_idx,
                t_idx + 3
            );
            let name = custom_names.get(&sig).cloned().unwrap_or(default_name);

            cards.push(MetaspriteEntity {
                entity_id: cards.len() + 100,
                signature_key: sig,
                name,
                bbox_x: 0,
                bbox_y: 0,
                width: 16,
                height: 16,
                primary_palette: 0,
                is_8x16_mode: is_8x16,
                pieces,
            });
        }

        t_idx += 4;
    }

    cards
}

/// Resuelve cualquier punto o rectángulo `(rx, ry, rw, rh)` seleccionado por el usuario sobre la
/// pantalla `256x240` (ya sean montañas, árboles, suelo, torretas, HUD o sprites) hacia sus tiles
/// `8x8` reales en `chr_8kb` y su paleta exacta, devolviendo un `MetaspriteEntity` editable en vivo (Fase G7).
pub fn resolve_screen_region_to_entity(
    screen_rgb: &[u8],
    chr_8kb: &[u8],
    palette_table: &[u8; 32],
    ppu_ctrl_bits: u8,
    rx: u16,
    ry: u16,
    rw: u16,
    rh: u16,
) -> Option<MetaspriteEntity> {
    if screen_rgb.len() < 256 * 240 * 3 || chr_8kb.len() < 8192 {
        return None;
    }

    // Alinear a rejilla de 8x8 y asegurar al menos 16x16 si fue un clic puntual
    let aligned_x = (rx & !7).min(248);
    let aligned_y = (ry & !7).min(232);
    let clean_w = ((rw.max(16) + 7) & !7).min(256 - aligned_x).max(8);
    let clean_h = ((rh.max(16) + 7) & !7).min(240 - aligned_y).max(8);

    let bg_table_base: u16 = if (ppu_ctrl_bits & 0x10) != 0 { 256 } else { 0 };
    let cols = (clean_w / 8) as usize;
    let rows = (clean_h / 8) as usize;

    // Pre-decodificar los 512 tiles de chr_8kb para comparación instantánea
    let mut decoded_tiles = Vec::with_capacity(512);
    for t in 0..512usize {
        let mut b = [0u8; 16];
        b.copy_from_slice(&chr_8kb[t * 16..t * 16 + 16]);
        decoded_tiles.push(AutoMemoryMapper::decode_tile_pixels(&b));
    }

    let mut pieces = Vec::with_capacity(cols * rows);
    let mut best_global_pal = 0u8;

    for r in 0..rows {
        for c in 0..cols {
            let sx = (aligned_x as usize) + c * 8;
            let sy = (aligned_y as usize) + r * 8;

            let (matched_tile, matched_pal, is_bg) = match_8x8_screen_patch_to_vram(
                screen_rgb,
                &decoded_tiles,
                palette_table,
                bg_table_base,
                sx,
                sy,
            );
            if r == 0 && c == 0 {
                best_global_pal = matched_pal;
            }

            pieces.push(MetaspritePiece {
                oam_index: 0,
                rel_x: (c * 8) as u16,
                rel_y: (r * 8) as u16,
                vram_tile_id: matched_tile,
                palette_idx: if is_bg {
                    matched_pal | 0x80
                } else {
                    matched_pal
                },
                flip_h: false,
                flip_v: false,
                behind_bg: is_bg,
            });
        }
    }

    let first_t = pieces.first().map(|p| p.vram_tile_id).unwrap_or(0);
    Some(MetaspriteEntity {
        entity_id: 500,
        signature_key: format!(
            "SCR_REG_{}_{}_{}x{}_T{:03X}",
            aligned_x, aligned_y, clean_w, clean_h, first_t
        ),
        name: format!(
            "Selección de Pantalla ({}x{} en X={}, Y={})",
            clean_w, clean_h, aligned_x, aligned_y
        ),
        bbox_x: aligned_x as u8,
        bbox_y: aligned_y as u8,
        width: clean_w,
        height: clean_h,
        primary_palette: best_global_pal,
        is_8x16_mode: false,
        pieces,
    })
}

fn match_8x8_screen_patch_to_vram(
    screen_rgb: &[u8],
    decoded_tiles: &[[[u8; 8]; 8]],
    palette_table: &[u8; 32],
    bg_table_base: u16,
    sx: usize,
    sy: usize,
) -> (u16, u8, bool) {
    let mut best_tile = bg_table_base;
    let mut best_pal = 0u8;
    let mut best_is_bg = true;
    let mut best_error = u64::MAX;

    // Primero buscar en la tabla de fondo activa (256 tiles) y luego en la otra tabla (256 tiles)
    let search_ranges = [
        (bg_table_base as usize, (bg_table_base as usize) + 256, true),
        (
            256 - (bg_table_base as usize),
            512 - (bg_table_base as usize),
            false,
        ),
    ];

    for &(start_t, end_t, is_bg_table) in &search_ranges {
        for pal_idx in 0..4u8 {
            let pal_base = if is_bg_table {
                (pal_idx as usize) * 4
            } else {
                16 + (pal_idx as usize) * 4
            };
            let pal_rgb: [[u8; 3]; 4] = [
                nes_color_rgb_u8(palette_table[0]),
                nes_color_rgb_u8(palette_table[(pal_base + 1) & 31]),
                nes_color_rgb_u8(palette_table[(pal_base + 2) & 31]),
                nes_color_rgb_u8(palette_table[(pal_base + 3) & 31]),
            ];

            for t_idx in start_t..end_t.min(decoded_tiles.len()) {
                let tile_px = &decoded_tiles[t_idx];
                let mut err = 0u64;
                for py in 0..8usize {
                    let y = (sy + py).min(239);
                    for px in 0..8usize {
                        let x = (sx + px).min(255);
                        let scr_off = (y * 256 + x) * 3;
                        let sr = screen_rgb[scr_off] as i32;
                        let sg = screen_rgb[scr_off + 1] as i32;
                        let sb = screen_rgb[scr_off + 2] as i32;

                        let c_idx = (tile_px[py][px] & 3) as usize;
                        let [tr, tg, tb] = pal_rgb[c_idx];
                        let dr = sr - (tr as i32);
                        let dg = sg - (tg as i32);
                        let db = sb - (tb as i32);
                        err += (dr * dr + dg * dg + db * db) as u64;
                        if err >= best_error {
                            break;
                        }
                    }
                    if err >= best_error {
                        break;
                    }
                }

                if err < best_error {
                    best_error = err;
                    best_tile = t_idx as u16;
                    best_pal = pal_idx;
                    best_is_bg = is_bg_table;
                    if best_error == 0 {
                        return (best_tile, best_pal, best_is_bg);
                    }
                }
            }
        }
    }

    (best_tile, best_pal, best_is_bg)
}
