use crate::project::{nes_color_rgb_u8, HdSpriteBinding, NgsProject};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct GraphicsEditorState {
    pub selected_tile: usize,
    pub active_brush_color: u8,
    pub metasprite_preview_mode: bool,
    pub status_msg: String,
}

impl Default for GraphicsEditorState {
    fn default() -> Self {
        Self {
            selected_tile: 0,
            active_brush_color: 3,
            metasprite_preview_mode: true,
            status_msg: String::new(),
        }
    }
}

impl GraphicsEditorState {
    pub fn invalidate_texture(&mut self) {
        // Mantenido por compatibilidad
    }

    pub fn ui(&mut self, _ctx: &egui::Context, ui: &mut egui::Ui, project: &mut NgsProject) {
        if !project.loaded {
            super::guide::welcome_ui(ui);
            return;
        }

        let total_tiles = project.total_tiles();
        if total_tiles == 0 {
            ui.label("Cargando gráficos del juego...");
            return;
        }
        if self.selected_tile >= total_tiles {
            self.selected_tile = 0;
        }

        // BARRA SUPERIOR: Explicación simple + Selector de paleta de colores
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("🎨 Paleta de Color:")
                    .strong()
                    .color(Color32::from_rgb(180, 220, 255)),
            );

            let pal_names = [
                "Acción / Cálido",
                "Héroe Azul (P1)",
                "Héroe Rojo (P2)",
                "Selva / Verde",
                "Metálico / Base",
                "Fuego / Explosión",
                "Monocromo / HUD",
                "Magia / Púrpura",
            ];

            for (idx, name) in pal_names.iter().enumerate() {
                let is_sel = project.selected_palette == idx;
                let pal = project.palettes[idx];
                let [r, g, b] = nes_color_rgb_u8(pal[1]);
                let btn = egui::Button::new(
                    egui::RichText::new(*name)
                        .size(11.0)
                        .color(if is_sel { Color32::WHITE } else { Color32::LIGHT_GRAY }),
                )
                .fill(if is_sel { Color32::from_rgb(r / 2, g / 2, b / 2) } else { Color32::from_rgb(25, 28, 35) })
                .stroke(if is_sel { Stroke::new(1.8, Color32::YELLOW) } else { Stroke::NONE });

                if ui.add(btn).clicked() {
                    project.selected_palette = idx;
                }
            }
        });
        ui.separator();

        let active_pal = project.palettes[project.selected_palette];

        // DOS COLUMNAS: Izquierda = Catálogo completo (scroll continuo), Derecha = Editor de píxeles
        ui.columns(2, |cols| {
            // COLUMNA 1: Galería Continua con TODOS los tiles del juego
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("🖼️ Catálogo de Gráficos de Todo el Juego");
                    ui.label(format!("({} tiles en total)", total_tiles));
                });
                ui.label(
                    egui::RichText::new("Haz scroll para ver todos los personajes, enemigos y bloques. Haz clic en cualquiera:")
                        .small()
                        .color(Color32::LIGHT_GRAY),
                );
                ui.add_space(4.0);

                let tile_draw_size = 22.0;
                let tiles_per_row = 16usize;
                let total_rows = (total_tiles + tiles_per_row - 1) / tiles_per_row;
                let grid_width = tile_draw_size * tiles_per_row as f32;
                let total_height = tile_draw_size * total_rows as f32;

                egui::ScrollArea::vertical()
                    .max_height(580.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(grid_width, total_height),
                            Sense::click(),
                        );
                        let painter = ui.painter_at(rect);
                        painter.rect_filled(rect, 2.0, Color32::from_rgb(14, 16, 22));

                        // Renderizar sólo los tiles que están visibles en la pantalla (culling)
                        let visible_rect = ui.clip_rect();
                        let start_row = (((visible_rect.min.y - rect.min.y) / tile_draw_size).floor() as isize).max(0) as usize;
                        let end_row = (((visible_rect.max.y - rect.min.y) / tile_draw_size).ceil() as usize).min(total_rows);

                        for row in start_row..end_row {
                            for col in 0..tiles_per_row {
                                let t_idx = row * tiles_per_row + col;
                                if t_idx >= total_tiles {
                                    break;
                                }

                                let origin = Pos2::new(
                                    rect.min.x + col as f32 * tile_draw_size,
                                    rect.min.y + row as f32 * tile_draw_size,
                                );

                                draw_tile_8x8(&painter, project, t_idx, origin, tile_draw_size / 8.0, &active_pal);

                                // Resaltar si tiene reemplazo HD o si está seleccionado
                                let tile_rect = Rect::from_min_size(origin, Vec2::splat(tile_draw_size));
                                if project.hd_sprites.contains_key(&t_idx) {
                                    painter.rect_stroke(tile_rect, 0.0, Stroke::new(1.5, Color32::from_rgb(0, 220, 255)));
                                }
                                if t_idx == self.selected_tile {
                                    painter.rect_stroke(tile_rect, 0.0, Stroke::new(2.2, Color32::YELLOW));
                                }
                            }
                        }

                        if response.clicked() {
                            if let Some(pos) = response.interact_pointer_pos() {
                                let col = ((pos.x - rect.min.x) / tile_draw_size).floor() as usize;
                                let row = ((pos.y - rect.min.y) / tile_draw_size).floor() as usize;
                                if col < tiles_per_row {
                                    let clicked_tile = row * tiles_per_row + col;
                                    if clicked_tile < total_tiles {
                                        self.selected_tile = clicked_tile;
                                    }
                                }
                            }
                        }
                    });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.button("💾 Exportar todo el catálogo a imagen .BMP").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}_graficos_completos.bmp", project.game_title))
                            .add_filter("Imagen BMP", &["bmp"])
                            .save_file()
                        {
                            match export_all_tiles_to_bmp(project, &path) {
                                Ok(_) => self.status_msg = format!("Exportado correctamente a {}", path.display()),
                                Err(e) => self.status_msg = format!("Error al exportar: {}", e),
                            }
                        }
                    }
                    if !self.status_msg.is_empty() {
                        ui.label(&self.status_msg);
                    }
                });
            });

            // COLUMNA 2: Editor del Sprite Seleccionado en Grande
            cols[1].group(|ui| {
                let tile_label = project
                    .global_tiles
                    .get(self.selected_tile)
                    .map(|t| t.label.as_str())
                    .unwrap_or("Tile");

                ui.heading(format!("✏️ Dibujo del Sprite #{}", self.selected_tile));
                ui.label(egui::RichText::new(tile_label).color(Color32::from_rgb(180, 220, 255)));
                ui.separator();

                // Selector de Color del Pincel
                ui.label("1. Elige el color del pincel:");
                ui.horizontal(|ui| {
                    for c_idx in 0..4u8 {
                        let nes_col = active_pal[c_idx as usize];
                        let [r, g, b] = nes_color_rgb_u8(nes_col);
                        let col32 = Color32::from_rgb(r, g, b);
                        let is_sel = self.active_brush_color == c_idx;

                        let tag = match c_idx {
                            0 => "C0 (Fondo)",
                            1 => "C1 (Color 1)",
                            2 => "C2 (Color 2)",
                            _ => "C3 (Color 3)",
                        };

                        let btn = egui::Button::new(
                            egui::RichText::new(tag)
                                .size(12.0)
                                .color(if (r as u16 + g as u16 + b as u16) > 380 { Color32::BLACK } else { Color32::WHITE }),
                        )
                        .fill(col32)
                        .stroke(if is_sel { Stroke::new(2.5, Color32::YELLOW) } else { Stroke::new(1.0, Color32::DARK_GRAY) });

                        if ui.add(btn).clicked() {
                            self.active_brush_color = c_idx;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label("2. Pinta los píxeles con el mouse:");

                ui.horizontal(|ui| {
                    // Lienzo de edición 8x8 amplio (26px por píxel = 208x208)
                    let px_scale = 26.0;
                    let canvas_sz = px_scale * 8.0;
                    let (c_rect, c_resp) = ui.allocate_exact_size(Vec2::splat(canvas_sz), Sense::click_and_drag());
                    let c_painter = ui.painter_at(c_rect);
                    c_painter.rect_filled(c_rect, 0.0, Color32::BLACK);

                    let pixels = project.get_tile_pixels(self.selected_tile);

                    for r in 0..8 {
                        for c in 0..8 {
                            let v = pixels[r][c] as usize & 3;
                            let [cr, cg, cb] = nes_color_rgb_u8(active_pal[v]);
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

                    ui.add_space(14.0);
                    // Vista previa Metasprite 16x16 (para ver cómo se conecta con el tile de al lado)
                    ui.vertical(|ui| {
                        ui.checkbox(&mut self.metasprite_preview_mode, "Metasprite 16×16");
                        if self.metasprite_preview_mode {
                            ui.label(egui::RichText::new("Vista junto a tiles vecinos:").small());
                            let preview_box = 104.0;
                            let (m_rect, _) = ui.allocate_exact_size(Vec2::splat(preview_box), Sense::hover());
                            let m_painter = ui.painter_at(m_rect);
                            m_painter.rect_filled(m_rect, 2.0, Color32::from_rgb(18, 18, 24));

                            let sub_scale = preview_box / 16.0;
                            let offsets = [
                                (0usize, 0.0, 0.0),
                                (1usize, preview_box * 0.5, 0.0),
                                (16usize, 0.0, preview_box * 0.5),
                                (17usize, preview_box * 0.5, preview_box * 0.5),
                            ];
                            for (off, ox, oy) in offsets {
                                let neighbor = (self.selected_tile + off).min(total_tiles.saturating_sub(1));
                                draw_tile_8x8(
                                    &m_painter,
                                    project,
                                    neighbor,
                                    Pos2::new(m_rect.min.x + ox, m_rect.min.y + oy),
                                    sub_scale,
                                    &active_pal,
                                );
                            }
                        }
                    });
                });

                ui.separator();
                ui.heading("✨ Remasterizar con Imagen HD (32-bit RGBA)");
                ui.label("Puedes reemplazar este sprite por un dibujo en alta definición (.PNG o .BMP):");
                ui.add_space(4.0);

                if ui.button("📂 Vincular Imagen HD a este Sprite").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Imagen PNG o BMP", &["png", "bmp"])
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
                        self.status_msg = format!("Imagen HD vinculada al Sprite #{}", self.selected_tile);
                    }
                }

                if let Some(binding) = project.hd_sprites.get(&self.selected_tile) {
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 255),
                        format!("✔ Archivo HD activo: {}", binding.bmp_or_rgba_path),
                    );
                    if ui.button("🗑 Quitar reemplazo HD").clicked() {
                        project.hd_sprites.remove(&self.selected_tile);
                    }
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

fn export_all_tiles_to_bmp(project: &NgsProject, path: &Path) -> Result<(), String> {
    let total = project.total_tiles();
    if total == 0 {
        return Err("No hay tiles para exportar.".to_string());
    }

    let tiles_per_row = 16usize;
    let rows = (total + tiles_per_row - 1) / tiles_per_row;
    let width = (tiles_per_row * 8) as u32;
    let height = (rows * 8) as u32;
    let row_bytes = ((width * 3 + 3) / 4) * 4;
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

    for y in (0..height as usize).rev() {
        let ty = y / 8;
        let py = y % 8;
        let mut line = Vec::with_capacity(row_bytes as usize);
        for x in 0..width as usize {
            let tx = x / 8;
            let px = x % 8;
            let t_idx = ty * tiles_per_row + tx;
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
        while line.len() % 4 != 0 {
            line.push(0);
        }
        f.write_all(&line).map_err(|e| e.to_string())?;
    }
    Ok(())
}
