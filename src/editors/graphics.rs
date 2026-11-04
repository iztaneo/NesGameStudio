use crate::project::{nes_color_rgb_u8, HdSpriteBinding, NgsProject};
use eframe::egui::{self, Color32, ColorImage, Pos2, Rect, Sense, Stroke, TextureHandle, TextureOptions, Vec2};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct GraphicsEditorState {
    pub current_table_page: usize, // 0 = Tabla 0 ($0000-$0FFF: 256 tiles), 1 = Tabla 1 ($1000-$1FFF: 256 tiles)
    pub selected_tile: usize,
    pub active_brush_color: u8,
    pub status_msg: String,
    scene_tex: Option<TextureHandle>,
    cached_scene_idx: usize,
}

impl Default for GraphicsEditorState {
    fn default() -> Self {
        Self {
            current_table_page: 0,
            selected_tile: 0,
            active_brush_color: 3,
            status_msg: String::new(),
            scene_tex: None,
            cached_scene_idx: usize::MAX,
        }
    }
}

impl GraphicsEditorState {
    pub fn invalidate_texture(&mut self) {
        self.cached_scene_idx = usize::MAX;
    }

    pub fn ui(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, project: &mut NgsProject) {
        if !project.loaded {
            super::guide::welcome_ui(ui);
            return;
        }

        super::guide::tab_hint(
            ui,
            "Elige una escena arriba → haz clic en un personaje (columna 1) o en un cuadro de la hoja (columna 2) \
             → píntalo en la columna 3. Usa los botones BG/SP para cambiar los colores.",
        );

        // Selector de Escena Descomprimida (Título, Nivel 1, o Avanzar el juego en vivo)
        ui.horizontal_wrapped(|ui| {
            ui.label("🎬 Escena / Banco Descomprimido:");
            egui::ComboBox::from_id_salt("scene_selector")
                .selected_text(
                    project
                        .scenes
                        .get(project.active_scene_idx)
                        .map(|s| s.title.as_str())
                        .unwrap_or("Escena"),
                )
                .width(430.0)
                .show_ui(ui, |ui| {
                    for idx in 0..project.scenes.len() {
                        let title = project.scenes[idx].title.clone();
                        if ui
                            .selectable_value(&mut project.active_scene_idx, idx, title)
                            .clicked()
                        {
                            self.invalidate_texture();
                        }
                    }
                });

            if ui.button("⏭ Avanzar Juego +60 Frames y Capturar Mapas de Bits").clicked() {
                for _ in 0..60 {
                    project.nes_live.step_frame();
                    project.nes_live.bus.apu.audio_samples.clear();
                }
                let n = project.scenes.len() + 1;
                project.capture_current_live_scene(format!(
                    "Escena {}: Captura en Vivo (+{}s)",
                    n,
                    n
                ));
                self.invalidate_texture();
                self.status_msg = "Nueva escena de CHR-RAM y Paletas capturada desde la CPU 6502.".to_string();
            }
        });
        ui.separator();

        let total_tiles = project.total_tiles().max(1);
        let total_pages = (total_tiles + 255) / 256;
        if self.current_table_page >= total_pages {
            self.current_table_page = 0;
        }

        ui.columns(3, |cols| {
            // COLUMNA 1: Vista de la Escena Real 256x240 + Lista de Sprites OAM Reales en Pantalla
            cols[0].group(|ui| {
                ui.label(egui::RichText::new("1. Vista de Escena Real y Sprites Activos (OAM)").strong());
                if let Some(sc) = project.scenes.get(project.active_scene_idx) {
                    if self.cached_scene_idx != project.active_scene_idx || self.scene_tex.is_none() {
                        if sc.screen_rgb.len() >= 256 * 240 * 3 {
                            let img = ColorImage::from_rgb([256, 240], &sc.screen_rgb[..256 * 240 * 3]);
                            self.scene_tex = Some(ctx.load_texture(
                                "ngs_scene_preview",
                                img,
                                TextureOptions::NEAREST,
                            ));
                            self.cached_scene_idx = project.active_scene_idx;
                        }
                    }
                }

                if let Some(tex) = &self.scene_tex {
                    ui.add(egui::Image::new(tex).fit_to_exact_size(Vec2::new(256.0, 240.0)));
                }

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("🎯 Haz clic en cualquier Sprite activo de la escena para ir a su Mapa de Bits:")
                        .small(),
                );

                // Extraer los sprites visibles del OAM de la escena actual
                let mut visible_sprites = Vec::new();
                if let Some(sc) = project.scenes.get(project.active_scene_idx) {
                    let sprite_8x16 = (sc.ppu_ctrl_bits & 0x20) != 0;
                    let sp_table_base = if (sc.ppu_ctrl_bits & 0x08) != 0 { 256usize } else { 0usize };
                    for sp_idx in 0..64 {
                        let y = sc.oam[sp_idx * 4];
                        let tile_byte = sc.oam[sp_idx * 4 + 1] as usize;
                        let attr = sc.oam[sp_idx * 4 + 2];
                        let x = sc.oam[sp_idx * 4 + 3];
                        if y < 239 && (x > 0 || y > 0) {
                            let (real_tile, pal_idx) = if sprite_8x16 {
                                let bank = (tile_byte & 1) * 256;
                                (bank + (tile_byte & 0xFE), 4 + ((attr & 3) as usize))
                            } else {
                                (sp_table_base + tile_byte, 4 + ((attr & 3) as usize))
                            };
                            visible_sprites.push((sp_idx, x, y, real_tile, pal_idx, sprite_8x16));
                        }
                    }
                }

                egui::ScrollArea::vertical().max_height(175.0).show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        for (sp_idx, x, y, real_tile, pal_idx, is_8x16) in visible_sprites {
                            let pal = project.active_palette_4(pal_idx);
                            let sz = Vec2::new(28.0, if is_8x16 { 44.0 } else { 28.0 });
                            let (r, resp) = ui.allocate_exact_size(sz, Sense::click());
                            let p = ui.painter_at(r);
                            p.rect_filled(r, 2.0, Color32::from_rgb(24, 28, 36));
                            draw_tile_8x8(
                                &p,
                                project,
                                real_tile.min(total_tiles - 1),
                                Pos2::new(r.min.x + 2.0, r.min.y + 2.0),
                                3.0,
                                &pal,
                            );
                            if is_8x16 {
                                draw_tile_8x8(
                                    &p,
                                    project,
                                    (real_tile + 1).min(total_tiles - 1),
                                    Pos2::new(r.min.x + 2.0, r.min.y + 26.0),
                                    2.2,
                                    &pal,
                                );
                            }
                            if self.selected_tile == real_tile {
                                p.rect_stroke(r, 2.0, Stroke::new(1.5, Color32::YELLOW));
                            }
                            if resp
                                .on_hover_text(format!(
                                    "Sprite OAM #{} en ({},{}) -> Tile #{:03X}, Paleta Sprite #{}",
                                    sp_idx,
                                    x,
                                    y,
                                    real_tile,
                                    pal_idx - 4
                                ))
                                .clicked()
                            {
                                self.current_table_page = real_tile / 256;
                                self.selected_tile = real_tile;
                                project.selected_palette = pal_idx;
                            }
                        }
                    });
                });
            });

            // COLUMNA 2: Hoja Completa de Mapas de Bits (Tabla 0: Sprites $0000-$0FFF | Tabla 1: Escenario $1000-$1FFF)
            cols[1].group(|ui| {
                ui.label(egui::RichText::new("2. Tablas de Mapas de Bits (CHR Descomprimida)").strong());
                ui.horizontal(|ui| {
                    for page in 0..total_pages {
                        let label = match page {
                            0 => "Tabla 0 ($0000: Sprites / Letras)",
                            1 => "Tabla 1 ($1000: Bloques / Escenario)",
                            _ => "Banco Extra",
                        };
                        if ui
                            .selectable_label(self.current_table_page == page, format!("{} (#{})", label, page))
                            .clicked()
                        {
                            self.current_table_page = page;
                        }
                    }
                });

                // Selector de las 8 Paletas Reales del Juego (0..3 = Fondo, 4..7 = Sprites)
                ui.horizontal_wrapped(|ui| {
                    for pal_idx in 0..8 {
                        let is_sel = project.selected_palette == pal_idx;
                        let name = if pal_idx < 4 {
                            format!("BG{}", pal_idx)
                        } else {
                            format!("SP{}", pal_idx - 4)
                        };
                        if ui.selectable_label(is_sel, name).clicked() {
                            project.selected_palette = pal_idx;
                        }
                    }
                });

                ui.add_space(4.0);
                let tile_draw_size = 18.0;
                let grid_px = tile_draw_size * 16.0;
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(grid_px), Sense::click());
                let painter = ui.painter_at(rect);
                painter.rect_filled(rect, 2.0, Color32::from_rgb(16, 16, 22));

                let pal = project.active_palette_4(project.selected_palette);
                let base_tile = self.current_table_page * 256;

                for ty in 0..16 {
                    for tx in 0..16 {
                        let t_idx = base_tile + ty * 16 + tx;
                        if t_idx >= total_tiles {
                            continue;
                        }
                        let origin = Pos2::new(
                            rect.min.x + tx as f32 * tile_draw_size,
                            rect.min.y + ty as f32 * tile_draw_size,
                        );
                        draw_tile_8x8(&painter, project, t_idx, origin, tile_draw_size / 8.0, &pal);

                        if project.hd_sprites.contains_key(&t_idx) {
                            let r = Rect::from_min_size(origin, Vec2::splat(tile_draw_size));
                            painter.rect_stroke(r, 0.0, Stroke::new(1.5, Color32::from_rgb(0, 220, 255)));
                        }
                        if t_idx == self.selected_tile {
                            let r = Rect::from_min_size(origin, Vec2::splat(tile_draw_size));
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
                if ui.button("💾 Exportar esta Tabla de Mapas de Bits a .BMP").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&format!("{}_tabla_{}.bmp", project.game_title, self.current_table_page))
                        .add_filter("BMP Image", &["bmp"])
                        .save_file()
                    {
                        match export_bank_to_bmp(project, self.current_table_page, &path) {
                            Ok(_) => self.status_msg = format!("Tabla exportada a {}", path.display()),
                            Err(e) => self.status_msg = format!("Error: {}", e),
                        }
                    }
                }
            });

            // COLUMNA 3: Editor de Píxeles del Mapa de Bits Seleccionado + Vínculo HD
            cols[2].group(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "3. Editor de Píxeles — Tile #{:03X} (Índice {})",
                        self.selected_tile, self.selected_tile
                    ))
                    .strong(),
                );

                let pal = project.active_palette_4(project.selected_palette);
                ui.horizontal(|ui| {
                    ui.label("Pincel:");
                    for c_idx in 0..4u8 {
                        let nes_col = pal[c_idx as usize];
                        let [r, g, b] = nes_color_rgb_u8(nes_col);
                        let col32 = Color32::from_rgb(r, g, b);
                        let is_sel = self.active_brush_color == c_idx;
                        let btn = egui::Button::new(
                            egui::RichText::new(format!(" C{} (${:02X}) ", c_idx, nes_col))
                                .color(if (r as u16 + g as u16 + b as u16) > 380 { Color32::BLACK } else { Color32::WHITE }),
                        )
                        .fill(col32)
                        .stroke(if is_sel { Stroke::new(2.0, Color32::YELLOW) } else { Stroke::NONE });
                        if ui.add(btn).clicked() {
                            self.active_brush_color = c_idx;
                        }
                    }
                });

                ui.add_space(6.0);
                let px_scale = 21.0;
                let canvas_sz = px_scale * 8.0;
                let (c_rect, c_resp) = ui.allocate_exact_size(Vec2::splat(canvas_sz), Sense::click_and_drag());
                let c_painter = ui.painter_at(c_rect);
                let pixels = project.get_tile_pixels(self.selected_tile);

                for r in 0..8 {
                    for c in 0..8 {
                        let v = pixels[r][c] as usize & 3;
                        let [cr, cg, cb] = nes_color_rgb_u8(pal[v]);
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

                ui.separator();
                ui.label(egui::RichText::new("✨ Reemplazo por Textura HD (Enhanced 2D / Hybrid)").strong());
                if ui.button("📂 Asociar Imagen HD (.BMP / .PNG) a este Tile").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Imagen BMP/PNG", &["bmp", "png"])
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
                        self.status_msg = format!("Vinculado Sprite HD a Tile #{:03X}", self.selected_tile);
                    }
                }
                if let Some(binding) = project.hd_sprites.get(&self.selected_tile) {
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 255),
                        format!("✔ Vinculado: {}", binding.bmp_or_rgba_path),
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
            let [cr, cg, cb] = nes_color_rgb_u8(pal[v]);
            let p_rect = Rect::from_min_size(
                Pos2::new(origin.x + c as f32 * px_size, origin.y + r as f32 * px_size),
                Vec2::splat(px_size),
            );
            painter.rect_filled(p_rect, 0.0, Color32::from_rgb(cr, cg, cb));
        }
    }
}

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

    let pal = project.active_palette_4(project.selected_palette);
    let base_tile = bank_page * 256;
    let total = project.total_tiles();

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
            let [r, g, b] = nes_color_rgb_u8(pal[v]);
            line.push(b);
            line.push(g);
            line.push(r);
        }
        f.write_all(&line).map_err(|e| e.to_string())?;
    }
    Ok(())
}
