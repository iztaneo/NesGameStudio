use crate::project::{HdSpriteBinding, NgsProject, NES_PALETTE_RGB};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct GraphicsEditorState {
    pub current_bank_page: usize, // Cada página muestra 256 tiles (4 KB)
    pub selected_tile: usize,
    pub active_brush_color: u8, // 0..=3
    pub metasprite_mode_16x16: bool,
    pub status_msg: String,
}

impl Default for GraphicsEditorState {
    fn default() -> Self {
        Self {
            current_bank_page: 0,
            selected_tile: 0,
            active_brush_color: 3,
            metasprite_mode_16x16: true,
            status_msg: String::new(),
        }
    }
}

impl GraphicsEditorState {
    pub fn ui(&mut self, ui: &mut egui::Ui, project: &mut NgsProject) {
        let total_tiles = project.total_tiles().max(1);
        let total_pages = (total_tiles + 255) / 256;
        if self.current_bank_page >= total_pages {
            self.current_bank_page = 0;
        }

        ui.horizontal(|ui| {
            ui.heading("🎨 Galería Total de Gráficos (CHR-ROM / CHR-RAM) & Sprites HD");
            ui.separator();
            ui.checkbox(&mut project.widescreen_16_9, "🖥️ Activar Widescreen 16:9 Real (426x240)");
        });
        ui.separator();

        ui.columns(2, |cols| {
            // COLUMNA IZQUIERDA: Navegador de Bancos Gráficos (Cuadrícula 16x16 tiles = 256 por página)
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.label(format!(
                        "Banco Gráfico ({}) — Página {}/{}:",
                        if project.is_chr_ram { "CHR-RAM desde PRG" } else { "CHR-ROM" },
                        self.current_bank_page + 1,
                        total_pages
                    ));
                    if ui.button("◀ Anterior").clicked() && self.current_bank_page > 0 {
                        self.current_bank_page -= 1;
                    }
                    if ui.button("Siguiente ▶").clicked() && self.current_bank_page + 1 < total_pages {
                        self.current_bank_page += 1;
                    }
                });

                ui.add_space(4.0);
                // Selector de las 8 Paletas
                ui.horizontal_wrapped(|ui| {
                    ui.label("Paleta activa:");
                    for pal_idx in 0..8 {
                        let is_sel = project.selected_palette == pal_idx;
                        let btn = ui.selectable_label(is_sel, format!("Pal #{}", pal_idx));
                        if btn.clicked() {
                            project.selected_palette = pal_idx;
                        }
                    }
                });

                ui.add_space(6.0);
                // Dibujar lienzo de 16x16 tiles (cada tile a escala 2.5x = 20x20 px -> 320x320 px total)
                let tile_draw_size = 20.0;
                let grid_px = tile_draw_size * 16.0;
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(grid_px), Sense::click());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 2.0, Color32::from_rgb(18, 18, 24));

                let pal = project.palettes[project.selected_palette];
                let base_tile = self.current_bank_page * 256;

                for ty in 0..16 {
                    for tx in 0..16 {
                        let t_idx = base_tile + ty * 16 + tx;
                        if t_idx >= total_tiles {
                            continue;
                        }
                        let tile_origin = Pos2::new(
                            rect.min.x + tx as f32 * tile_draw_size,
                            rect.min.y + ty as f32 * tile_draw_size,
                        );
                        draw_tile_8x8(
                            &painter,
                            project,
                            t_idx,
                            tile_origin,
                            tile_draw_size / 8.0,
                            &pal,
                        );

                        // Resaltar si tiene reemplazo HD o si está seleccionado
                        if project.hd_sprites.contains_key(&t_idx) {
                            let r = Rect::from_min_size(tile_origin, Vec2::splat(tile_draw_size));
                            painter.rect_stroke(r, 0.0, Stroke::new(1.5, Color32::from_rgb(0, 220, 255)));
                        }
                        if t_idx == self.selected_tile {
                            let r = Rect::from_min_size(tile_origin, Vec2::splat(tile_draw_size));
                            painter.rect_stroke(r, 0.0, Stroke::new(2.0, Color32::YELLOW));
                        }
                    }
                }

                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        let gx = ((pos.x - rect.min.x) / tile_draw_size).floor() as usize;
                        let gy = ((pos.y - rect.min.y) / tile_draw_size).floor() as usize;
                        if gx < 16 && gy < 16 {
                            let chosen = base_tile + gy * 16 + gx;
                            if chosen < total_tiles {
                                self.selected_tile = chosen;
                            }
                        }
                    }
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("💾 Exportar Banco Actual a .BMP").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}_bank_{}.bmp", project.game_title, self.current_bank_page))
                            .add_filter("BMP Image", &["bmp"])
                            .save_file()
                        {
                            match export_bank_to_bmp(project, self.current_bank_page, &path) {
                                Ok(_) => self.status_msg = format!("Exportado a {}", path.display()),
                                Err(e) => self.status_msg = format!("Error: {}", e),
                            }
                        }
                    }
                    ui.label(format!("Tiles editados: {} | Sprites HD: {}", project.modified_tiles.len(), project.hd_sprites.len()));
                });
            });

            // COLUMNA DERECHA: Editor de Píxeles 8x8, Armador de Metasprite 16x16 y Reemplazo HD
            cols[1].group(|ui| {
                ui.heading(format!("🔍 Editor de Tile #{:04X} (Índice {})", self.selected_tile, self.selected_tile));
                ui.add_space(4.0);

                // Selector de color de pincel (0..=3)
                let pal = project.palettes[project.selected_palette];
                ui.horizontal(|ui| {
                    ui.label("Color de Pincel (0..3):");
                    for c_idx in 0..4u8 {
                        let nes_col = pal[c_idx as usize] as usize & 63;
                        let [r, g, b] = NES_PALETTE_RGB[nes_col];
                        let col32 = Color32::from_rgb(r, g, b);
                        let is_sel = self.active_brush_color == c_idx;
                        let btn = egui::Button::new(
                            egui::RichText::new(format!(" C{} (${:02X}) ", c_idx, nes_col))
                                .color(if (r as u16 + g as u16 + b as u16) > 380 { Color32::BLACK } else { Color32::WHITE }),
                        )
                        .fill(col32)
                        .stroke(if is_sel { Stroke::new(2.5, Color32::YELLOW) } else { Stroke::NONE });
                        if ui.add(btn).clicked() {
                            self.active_brush_color = c_idx;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    // Lienzo de edición 8x8 (escala 22px por píxel = 176x176)
                    let px_scale = 22.0;
                    let canvas_sz = px_scale * 8.0;
                    let (c_rect, c_resp) = ui.allocate_exact_size(Vec2::splat(canvas_sz), Sense::click_and_drag());
                    let c_painter = ui.painter_at(c_rect);
                    let pixels = project.get_tile_pixels(self.selected_tile);

                    for r in 0..8 {
                        for c in 0..8 {
                            let v = pixels[r][c] as usize & 3;
                            let nes_col = pal[v] as usize & 63;
                            let [cr, cg, cb] = NES_PALETTE_RGB[nes_col];
                            let p_rect = Rect::from_min_size(
                                Pos2::new(c_rect.min.x + c as f32 * px_scale, c_rect.min.y + r as f32 * px_scale),
                                Vec2::splat(px_scale - 1.0),
                            );
                            c_painter.rect_filled(p_rect, 0.0, Color32::from_rgb(cr, cg, cb));
                        }
                    }

                    if c_resp.clicked() || c_resp.dragged() {
                        if let Some(pos) = c_resp.interact_pointer_pos() {
                            let col = ((pos.x - c_rect.min.x) / px_scale).floor() as usize;
                            let row = ((pos.y - c_rect.min.y) / px_scale).floor() as usize;
                            if row < 8 && col < 8 {
                                project.set_tile_pixel(self.selected_tile, row, col, self.active_brush_color);
                            }
                        }
                    }

                    ui.add_space(12.0);
                    // Vista Previa de Metasprite 16x16 (4 tiles vecinos armados)
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.metasprite_mode_16x16, "Vista Metasprite 16×16");
                        let meta_sz = 96.0;
                        let (m_rect, _) = ui.allocate_exact_size(Vec2::splat(meta_sz), Sense::hover());
                        let m_painter = ui.painter_at(m_rect);
                        m_painter.rect_filled(m_rect, 2.0, Color32::BLACK);
                        if self.metasprite_mode_16x16 {
                            let sub = meta_sz / 16.0;
                            let offsets = [
                                (0, 0.0, 0.0),
                                (1, meta_sz * 0.5, 0.0),
                                (16, 0.0, meta_sz * 0.5),
                                (17, meta_sz * 0.5, meta_sz * 0.5),
                            ];
                            for (off, ox, oy) in offsets {
                                draw_tile_8x8(
                                    &m_painter,
                                    project,
                                    (self.selected_tile + off).min(total_tiles - 1),
                                    Pos2::new(m_rect.min.x + ox, m_rect.min.y + oy),
                                    sub,
                                    &pal,
                                );
                            }
                        }
                    });
                });

                ui.separator();
                ui.heading("✨ Reemplazo por Sprite HD (32-bit RGBA / Enhanced 2D)");
                ui.label("Asocia una textura externa (.BMP / .PNG) a este Tile para el modo Enhanced 2D / Hybrid:");
                if ui.button("📂 Vincular Imagen HD (.BMP) a este Tile").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Imagen BMP/RGBA", &["bmp", "png"])
                        .pick_file()
                    {
                        project.hd_sprites.insert(
                            self.selected_tile,
                            HdSpriteBinding {
                                tile_index: self.selected_tile,
                                width: 16,
                                height: 16,
                                bmp_or_rgba_path: path.display().to_string(),
                                rgba_pixels: vec![0xFF00E5FF; 256],
                            },
                        );
                        self.status_msg = format!("Vinculado Sprite HD a Tile #{:04X}", self.selected_tile);
                    }
                }
                if let Some(binding) = project.hd_sprites.get(&self.selected_tile) {
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 255),
                        format!("✔ Vinculado a: {}", binding.bmp_or_rgba_path),
                    );
                }
                if !self.status_msg.is_empty() {
                    ui.colored_label(Color32::LIGHT_GREEN, &self.status_msg);
                }
            });
        });
    }
}

fn draw_tile_8x8(
    painter: &egui::Painter,
    project: &NgsProject,
    tile_idx: usize,
    origin: Pos2,
    px_size: f32,
    pal: &[u8; 4],
) {
    let pixels = project.get_tile_pixels(tile_idx);
    for r in 0..8 {
        for c in 0..8 {
            let v = pixels[r][c] as usize & 3;
            let nes_col = pal[v] as usize & 63;
            let [cr, cg, cb] = NES_PALETTE_RGB[nes_col];
            let p_rect = Rect::from_min_size(
                Pos2::new(origin.x + c as f32 * px_size, origin.y + r as f32 * px_size),
                Vec2::splat(px_size),
            );
            painter.rect_filled(p_rect, 0.0, Color32::from_rgb(cr, cg, cb));
        }
    }
}

/// Exporta los 256 tiles de una página (128x128 píxeles) a un archivo .BMP estándar de 24 bits
fn export_bank_to_bmp(project: &NgsProject, bank_page: usize, path: &Path) -> Result<(), String> {
    let width = 128u32;
    let height = 128u32;
    let row_bytes = width * 3;
    let data_size = row_bytes * height;
    let file_size = 54 + data_size;

    let mut f = File::create(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 54];
    header[0] = b'B';
    header[1] = b'M';
    header[2..6].copy_from_slice(&file_size.to_le_bytes());
    header[10..14].copy_from_slice(&54u32.to_le_bytes());
    header[14..18].copy_from_slice(&40u32.to_le_bytes());
    header[18..22].copy_from_slice(&width.to_le_bytes());
    header[22..26].copy_from_slice(&height.to_le_bytes());
    header[26..28].copy_from_slice(&1u16.to_le_bytes());
    header[28..30].copy_from_slice(&24u16.to_le_bytes());
    header[34..38].copy_from_slice(&data_size.to_le_bytes());
    f.write_all(&header).map_err(|e| e.to_string())?;

    let pal = project.palettes[project.selected_palette];
    let base_tile = bank_page * 256;
    let total = project.total_tiles();

    // BMP almacena las filas de abajo hacia arriba
    for y in (0..128usize).rev() {
        let ty = y / 8;
        let py = y % 8;
        let mut line = Vec::with_capacity(row_bytes as usize);
        for x in 0..128usize {
            let tx = x / 8;
            let px = x % 8;
            let t_idx = base_tile + ty * 16 + tx;
            let v = if t_idx < total {
                project.get_tile_pixels(t_idx)[py][px] as usize & 3
            } else {
                0
            };
            let [r, g, b] = NES_PALETTE_RGB[pal[v] as usize & 63];
            line.push(b);
            line.push(g);
            line.push(r);
        }
        f.write_all(&line).map_err(|e| e.to_string())?;
    }
    Ok(())
}
