use crate::project::{nes_color_rgb_u8, HdSpriteBinding, NgsProject};
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};
use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum SpriteSizeMode {
    Character16x32, // 2x4 tiles (16x32 px) — Bill, Lance, Soldados
    Block16x16,     // 2x2 tiles (16x16 px) — Bloques, Ítems, Soldado Agachado
    Boss32x32,      // 4x4 tiles (32x32 px) — Jefes, Torretas y Red Falcon
    Tile8x8,        // 1x1 tile (8x8 px)   — Tile individual
}

impl SpriteSizeMode {
    pub fn dimensions(&self) -> (usize, usize, usize, usize) {
        // (tiles_w, tiles_h, width_px, height_px)
        match self {
            SpriteSizeMode::Character16x32 => (2, 4, 16, 32),
            SpriteSizeMode::Block16x16 => (2, 2, 16, 16),
            SpriteSizeMode::Boss32x32 => (4, 4, 32, 32),
            SpriteSizeMode::Tile8x8 => (1, 1, 8, 8),
        }
    }

    pub fn total_tiles(&self) -> usize {
        let (tw, th, _, _) = self.dimensions();
        tw * th
    }

    pub fn name(&self) -> &'static str {
        match self {
            SpriteSizeMode::Character16x32 => "🧍 Personaje Completo (16×32: Bill / Lance)",
            SpriteSizeMode::Block16x16 => "🧱 Bloque / Ítem (16×16)",
            SpriteSizeMode::Boss32x32 => "👾 Jefe / Torreta (32×32)",
            SpriteSizeMode::Tile8x8 => "🔲 Tile Individual (8×8)",
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum TileLayout {
    HorizontalRows,  // Fila 0: Tile 0, 1 | Fila 1: Tile 2, 3...
    VerticalColumns, // Columna 0: Tile 0..3 | Columna 1: Tile 4..7 (Sprites NES 8x16)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ToolMode {
    Pencil,
    Eraser,
    BucketFill,
}

pub struct GraphicsEditorState {
    pub selected_tile: usize,
    pub active_brush_color: u8,
    pub size_mode: SpriteSizeMode,
    pub layout: TileLayout,
    pub tool: ToolMode,
    pub selected_category: String,
    pub status_msg: String,
}

impl Default for GraphicsEditorState {
    fn default() -> Self {
        Self {
            selected_tile: 0,
            active_brush_color: 3,
            size_mode: SpriteSizeMode::Character16x32,
            layout: TileLayout::HorizontalRows,
            tool: ToolMode::Pencil,
            selected_category: "TODOS".to_string(),
            status_msg: String::new(),
        }
    }
}

impl GraphicsEditorState {
    pub fn invalidate_texture(&mut self) {}

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

        // BARRA SUPERIOR 1: Selector de Modo de Tamaño y Distribución
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("📐 Modo de Edición:")
                    .strong()
                    .color(Color32::from_rgb(180, 220, 255)),
            );

            for mode in [
                SpriteSizeMode::Character16x32,
                SpriteSizeMode::Block16x16,
                SpriteSizeMode::Boss32x32,
                SpriteSizeMode::Tile8x8,
            ] {
                let is_sel = self.size_mode == mode;
                if ui
                    .selectable_label(
                        is_sel,
                        egui::RichText::new(mode.name())
                            .strong()
                            .color(if is_sel { Color32::YELLOW } else { Color32::LIGHT_GRAY }),
                    )
                    .clicked()
                {
                    self.size_mode = mode;
                }
            }

            ui.separator();
            ui.label(egui::RichText::new("Disposición:").small());
            let is_h = self.layout == TileLayout::HorizontalRows;
            if ui.selectable_label(is_h, "Horizontal (Filas)").clicked() {
                self.layout = TileLayout::HorizontalRows;
            }
            if ui.selectable_label(!is_h, "Vertical (8×16 NES)").clicked() {
                self.layout = TileLayout::VerticalColumns;
            }
        });

        // BARRA SUPERIOR 2: Paleta de Color
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new("🎨 Paleta Activa:")
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
                "Magia / Alien",
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

        // DOS COLUMNAS: Izquierda = Catálogo Visual Completo, Derecha = Lienzo de Edición Metasprite
        ui.columns(2, |cols| {
            // =========================================================================
            // COLUMNA 1: CATÁLOGO COMPLETO DE GRÁFICOS (TODOS LOS BANCOS Y NIVELES)
            // =========================================================================
            cols[0].group(|ui| {
                ui.horizontal(|ui| {
                    ui.heading("🖼️ Catálogo de Gráficos del Juego");
                    ui.label(format!("({} tiles en total)", total_tiles));
                });

                // Filtro por Categorías / Bancos
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new("Filtro:").strong());

                    let mut categories = vec!["TODOS".to_string()];
                    for t in &project.global_tiles {
                        if !categories.contains(&t.category) {
                            categories.push(t.category.clone());
                        }
                    }

                    egui::ComboBox::from_id_salt("cat_filter_combo")
                        .selected_text(&self.selected_category)
                        .show_ui(ui, |ui| {
                            for cat in &categories {
                                ui.selectable_value(&mut self.selected_category, cat.clone(), cat);
                            }
                        });
                });
                ui.add_space(4.0);

                // Obtener los índices de tiles que coinciden con el filtro
                let matching_indices: Vec<usize> = (0..total_tiles)
                    .filter(|&i| {
                        if self.selected_category == "TODOS" {
                            true
                        } else {
                            project.global_tiles[i].category == self.selected_category
                        }
                    })
                    .collect();

                let step = match self.size_mode {
                    SpriteSizeMode::Character16x32 => 8,
                    SpriteSizeMode::Block16x16 => 4,
                    SpriteSizeMode::Boss32x32 => 16,
                    SpriteSizeMode::Tile8x8 => 1,
                };

                // Agrupar en entidades según el tamaño seleccionado
                let mut card_bases = Vec::new();
                let mut i = 0;
                while i < matching_indices.len() {
                    card_bases.push(matching_indices[i]);
                    i += step;
                }

                ui.label(
                    egui::RichText::new(format!(
                        "Mostrando {} entidades en modo {}",
                        card_bases.len(),
                        self.size_mode.name()
                    ))
                    .small()
                    .color(Color32::LIGHT_GRAY),
                );

                egui::ScrollArea::vertical()
                    .max_height(540.0)
                    .auto_shrink([false; 2])
                    .show(ui, |ui| {
                        match self.size_mode {
                            SpriteSizeMode::Character16x32 => {
                                // Grid de 16x32 (4 columnas de tarjetas de personajes)
                                let card_w = 48.0;
                                let card_h = 96.0;
                                let cols_count = 4;
                                let rows_count = (card_bases.len() + cols_count - 1) / cols_count;
                                let total_h = rows_count as f32 * (card_h + 8.0);

                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(cols_count as f32 * (card_w + 8.0), total_h),
                                    Sense::click(),
                                );
                                let painter = ui.painter_at(rect);

                                let visible_rect = ui.clip_rect();
                                let start_row = (((visible_rect.min.y - rect.min.y) / (card_h + 8.0)).floor() as isize).max(0) as usize;
                                let end_row = (((visible_rect.max.y - rect.min.y) / (card_h + 8.0)).ceil() as usize).min(rows_count);

                                for r in start_row..end_row {
                                    for c in 0..cols_count {
                                        let item_idx = r * cols_count + c;
                                        if item_idx >= card_bases.len() {
                                            break;
                                        }
                                        let base_t = card_bases[item_idx];
                                        let origin = Pos2::new(
                                            rect.min.x + c as f32 * (card_w + 8.0),
                                            rect.min.y + r as f32 * (card_h + 8.0),
                                        );
                                        let card_rect = Rect::from_min_size(origin, Vec2::new(card_w, card_h));

                                        let is_sel = base_t == self.selected_tile;
                                        painter.rect_filled(card_rect, 3.0, Color32::from_rgb(14, 16, 22));
                                        if is_sel {
                                            painter.rect_stroke(card_rect, 3.0, Stroke::new(2.0, Color32::YELLOW));
                                        }

                                        draw_metasprite(
                                            &painter,
                                            project,
                                            base_t,
                                            self.size_mode,
                                            self.layout,
                                            origin,
                                            card_w / 16.0,
                                            &active_pal,
                                        );

                                        let label = format!("#{:03X}", base_t);
                                        painter.text(
                                            Pos2::new(origin.x + 4.0, origin.y + card_h - 12.0),
                                            egui::Align2::LEFT_BOTTOM,
                                            label,
                                            egui::FontId::proportional(10.0),
                                            if is_sel { Color32::YELLOW } else { Color32::LIGHT_GRAY },
                                        );
                                    }
                                }

                                if response.clicked() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        let col = ((pos.x - rect.min.x) / (card_w + 8.0)).floor() as usize;
                                        let row = ((pos.y - rect.min.y) / (card_h + 8.0)).floor() as usize;
                                        if col < cols_count {
                                            let idx = row * cols_count + col;
                                            if idx < card_bases.len() {
                                                self.selected_tile = card_bases[idx];
                                            }
                                        }
                                    }
                                }
                            }
                            SpriteSizeMode::Block16x16 => {
                                // Grid de 16x16 (6 columnas)
                                let card_w = 44.0;
                                let card_h = 44.0;
                                let cols_count = 6;
                                let rows_count = (card_bases.len() + cols_count - 1) / cols_count;
                                let total_h = rows_count as f32 * (card_h + 6.0);

                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(cols_count as f32 * (card_w + 6.0), total_h),
                                    Sense::click(),
                                );
                                let painter = ui.painter_at(rect);

                                for (item_idx, &base_t) in card_bases.iter().enumerate() {
                                    let c = item_idx % cols_count;
                                    let r = item_idx / cols_count;
                                    let origin = Pos2::new(
                                        rect.min.x + c as f32 * (card_w + 6.0),
                                        rect.min.y + r as f32 * (card_h + 6.0),
                                    );
                                    let card_rect = Rect::from_min_size(origin, Vec2::new(card_w, card_h));
                                    let is_sel = base_t == self.selected_tile;

                                    painter.rect_filled(card_rect, 2.0, Color32::from_rgb(14, 16, 22));
                                    if is_sel {
                                        painter.rect_stroke(card_rect, 2.0, Stroke::new(2.0, Color32::YELLOW));
                                    }

                                    draw_metasprite(
                                        &painter,
                                        project,
                                        base_t,
                                        self.size_mode,
                                        self.layout,
                                        origin,
                                        card_w / 16.0,
                                        &active_pal,
                                    );
                                }

                                if response.clicked() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        let col = ((pos.x - rect.min.x) / (card_w + 6.0)).floor() as usize;
                                        let row = ((pos.y - rect.min.y) / (card_h + 6.0)).floor() as usize;
                                        if col < cols_count {
                                            let idx = row * cols_count + col;
                                            if idx < card_bases.len() {
                                                self.selected_tile = card_bases[idx];
                                            }
                                        }
                                    }
                                }
                            }
                            SpriteSizeMode::Boss32x32 => {
                                // Grid de Jefes 32x32 (3 columnas)
                                let card_w = 72.0;
                                let card_h = 72.0;
                                let cols_count = 3;
                                let rows_count = (card_bases.len() + cols_count - 1) / cols_count;
                                let total_h = rows_count as f32 * (card_h + 8.0);

                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(cols_count as f32 * (card_w + 8.0), total_h),
                                    Sense::click(),
                                );
                                let painter = ui.painter_at(rect);

                                for (item_idx, &base_t) in card_bases.iter().enumerate() {
                                    let c = item_idx % cols_count;
                                    let r = item_idx / cols_count;
                                    let origin = Pos2::new(
                                        rect.min.x + c as f32 * (card_w + 8.0),
                                        rect.min.y + r as f32 * (card_h + 8.0),
                                    );
                                    let card_rect = Rect::from_min_size(origin, Vec2::new(card_w, card_h));
                                    let is_sel = base_t == self.selected_tile;

                                    painter.rect_filled(card_rect, 2.0, Color32::from_rgb(14, 16, 22));
                                    if is_sel {
                                        painter.rect_stroke(card_rect, 2.0, Stroke::new(2.0, Color32::YELLOW));
                                    }

                                    draw_metasprite(
                                        &painter,
                                        project,
                                        base_t,
                                        self.size_mode,
                                        self.layout,
                                        origin,
                                        card_w / 32.0,
                                        &active_pal,
                                    );
                                }

                                if response.clicked() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        let col = ((pos.x - rect.min.x) / (card_w + 8.0)).floor() as usize;
                                        let row = ((pos.y - rect.min.y) / (card_h + 8.0)).floor() as usize;
                                        if col < cols_count {
                                            let idx = row * cols_count + col;
                                            if idx < card_bases.len() {
                                                self.selected_tile = card_bases[idx];
                                            }
                                        }
                                    }
                                }
                            }
                            SpriteSizeMode::Tile8x8 => {
                                // Grid tradicional 8x8 (16 columnas)
                                let tile_sz = 22.0;
                                let cols_count = 16usize;
                                let rows_count = (card_bases.len() + cols_count - 1) / cols_count;
                                let total_h = rows_count as f32 * tile_sz;

                                let (rect, response) = ui.allocate_exact_size(
                                    Vec2::new(cols_count as f32 * tile_sz, total_h),
                                    Sense::click(),
                                );
                                let painter = ui.painter_at(rect);

                                for (item_idx, &t_idx) in card_bases.iter().enumerate() {
                                    let c = item_idx % cols_count;
                                    let r = item_idx / cols_count;
                                    let origin = Pos2::new(
                                        rect.min.x + c as f32 * tile_sz,
                                        rect.min.y + r as f32 * tile_sz,
                                    );
                                    let is_sel = t_idx == self.selected_tile;

                                    draw_tile_8x8(&painter, project, t_idx, origin, tile_sz / 8.0, &active_pal);

                                    let t_rect = Rect::from_min_size(origin, Vec2::splat(tile_sz));
                                    if is_sel {
                                        painter.rect_stroke(t_rect, 0.0, Stroke::new(2.2, Color32::YELLOW));
                                    }
                                }

                                if response.clicked() {
                                    if let Some(pos) = response.interact_pointer_pos() {
                                        let col = ((pos.x - rect.min.x) / tile_sz).floor() as usize;
                                        let row = ((pos.y - rect.min.y) / tile_sz).floor() as usize;
                                        if col < cols_count {
                                            let idx = row * cols_count + col;
                                            if idx < card_bases.len() {
                                                self.selected_tile = card_bases[idx];
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.button("💾 Exportar todo el catálogo a .BMP").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(&format!("{}_catalogo_completo.bmp", project.game_title))
                            .add_filter("Imagen BMP", &["bmp"])
                            .save_file()
                        {
                            match export_all_tiles_to_bmp(project, &path) {
                                Ok(_) => self.status_msg = format!("Catálogo exportado a {}", path.display()),
                                Err(e) => self.status_msg = format!("Error: {}", e),
                            }
                        }
                    }
                    if !self.status_msg.is_empty() {
                        ui.label(&self.status_msg);
                    }
                });
            });

            // =========================================================================
            // COLUMNA 2: LIENZO DE DIBUJO CONTINUO METASPRITE (16x32, 16x16, 32x32, 8x8)
            // =========================================================================
            cols[1].group(|ui| {
                let (_, _, w_px, h_px) = self.size_mode.dimensions();
                let current_label = project
                    .global_tiles
                    .get(self.selected_tile)
                    .map(|t| t.label.as_str())
                    .unwrap_or("Tile");

                ui.heading(format!(
                    "✏️ Editor de {} (Tile Base #{:03X})",
                    self.size_mode.name(),
                    self.selected_tile
                ));
                ui.label(egui::RichText::new(current_label).color(Color32::from_rgb(180, 220, 255)));
                ui.separator();

                // 1. Selector de Herramientas y Colores
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Herramienta:").strong());
                    if ui.selectable_label(self.tool == ToolMode::Pencil, "✏️ Lápiz").clicked() {
                        self.tool = ToolMode::Pencil;
                    }
                    if ui.selectable_label(self.tool == ToolMode::Eraser, "🧹 Borrador").clicked() {
                        self.tool = ToolMode::Eraser;
                    }
                    if ui.selectable_label(self.tool == ToolMode::BucketFill, "🪣 Cubeta").clicked() {
                        self.tool = ToolMode::BucketFill;
                    }

                    ui.separator();
                    if ui.button("↔ Voltear X").clicked() {
                        flip_h_metasprite(project, self.selected_tile, self.size_mode, self.layout);
                        self.status_msg = "Sprite invertido horizontalmente.".to_string();
                    }
                    if ui.button("↕ Voltear Y").clicked() {
                        flip_v_metasprite(project, self.selected_tile, self.size_mode, self.layout);
                        self.status_msg = "Sprite invertido verticalmente.".to_string();
                    }
                    if ui.button("🗑 Limpiar").clicked() {
                        clear_metasprite(project, self.selected_tile, self.size_mode, self.layout);
                        self.status_msg = "Sprite limpiado a color de fondo.".to_string();
                    }
                });

                // Selector de Color del Pincel
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Color:").strong());
                    for c_idx in 0..4u8 {
                        let nes_col = active_pal[c_idx as usize];
                        let [r, g, b] = nes_color_rgb_u8(nes_col);
                        let col32 = Color32::from_rgb(r, g, b);
                        let is_sel = self.active_brush_color == c_idx;

                        let tag = match c_idx {
                            0 => "C0 (Fondo)",
                            1 => "C1 (Piel/Cuerpo)",
                            2 => "C2 (Ropa/Rifle)",
                            _ => "C3 (Detalles)",
                        };

                        let btn = egui::Button::new(
                            egui::RichText::new(tag)
                                .size(11.0)
                                .color(if (r as u16 + g as u16 + b as u16) > 380 { Color32::BLACK } else { Color32::WHITE }),
                        )
                        .fill(col32)
                        .stroke(if is_sel { Stroke::new(2.5, Color32::YELLOW) } else { Stroke::new(1.0, Color32::DARK_GRAY) });

                        if ui.add(btn).clicked() {
                            self.active_brush_color = c_idx;
                            self.tool = ToolMode::Pencil;
                        }
                    }
                });

                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("Pinta con el mouse directamente sobre la figura completa:")
                        .small()
                        .color(Color32::LIGHT_GRAY),
                );

                // 2. LIENZO INTERACTIVO (Dibuja continuamente sobre toda la figura)
                let px_scale = match self.size_mode {
                    SpriteSizeMode::Character16x32 => 16.0,
                    SpriteSizeMode::Block16x16 => 20.0,
                    SpriteSizeMode::Boss32x32 => 11.0,
                    SpriteSizeMode::Tile8x8 => 30.0,
                };

                let canvas_w = px_scale * w_px as f32;
                let canvas_h = px_scale * h_px as f32;

                let (c_rect, c_resp) = ui.allocate_exact_size(Vec2::new(canvas_w, canvas_h), Sense::click_and_drag());
                let c_painter = ui.painter_at(c_rect);
                c_painter.rect_filled(c_rect, 0.0, Color32::BLACK);

                // Dibujar píxeles actuales del metasprite
                let sprite_pixels = get_metasprite_pixels(project, self.selected_tile, self.size_mode, self.layout);

                for r in 0..h_px {
                    for c in 0..w_px {
                        let v = sprite_pixels[r][c] as usize & 3;
                        let [cr, cg, cb] = nes_color_rgb_u8(active_pal[v]);
                        let p_rect = Rect::from_min_size(
                            Pos2::new(c_rect.min.x + c as f32 * px_scale, c_rect.min.y + r as f32 * px_scale),
                            Vec2::splat(px_scale - 1.0),
                        );
                        c_painter.rect_filled(p_rect, 0.0, Color32::from_rgb(cr, cg, cb));
                    }
                }

                // Dibujar líneas de separación de tiles (cada 8 píxeles) para referencia
                for t_y in 1..(h_px / 8) {
                    let y = c_rect.min.y + (t_y * 8) as f32 * px_scale;
                    c_painter.line_segment(
                        [Pos2::new(c_rect.min.x, y), Pos2::new(c_rect.max.x, y)],
                        Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 120)),
                    );
                }
                for t_x in 1..(w_px / 8) {
                    let x = c_rect.min.x + (t_x * 8) as f32 * px_scale;
                    c_painter.line_segment(
                        [Pos2::new(x, c_rect.min.y), Pos2::new(x, c_rect.max.y)],
                        Stroke::new(1.5, Color32::from_rgba_unmultiplied(255, 255, 255, 120)),
                    );
                }

                // Interacción de dibujo con el mouse
                if c_resp.clicked() || c_resp.dragged() {
                    if let Some(pos) = c_resp.interact_pointer_pos() {
                        let col = ((pos.x - c_rect.min.x) / px_scale).floor() as isize;
                        let row = ((pos.y - c_rect.min.y) / px_scale).floor() as isize;

                        if col >= 0 && col < w_px as isize && row >= 0 && row < h_px as isize {
                            let draw_color = match self.tool {
                                ToolMode::Pencil => self.active_brush_color,
                                ToolMode::Eraser => 0,
                                ToolMode::BucketFill => self.active_brush_color,
                            };

                            if self.tool == ToolMode::BucketFill && c_resp.clicked() {
                                flood_fill_metasprite(
                                    project,
                                    self.selected_tile,
                                    self.size_mode,
                                    self.layout,
                                    col as usize,
                                    row as usize,
                                    draw_color,
                                );
                            } else if self.tool != ToolMode::BucketFill {
                                set_metasprite_pixel(
                                    project,
                                    self.selected_tile,
                                    self.size_mode,
                                    self.layout,
                                    col as usize,
                                    row as usize,
                                    draw_color,
                                );
                            }
                        }
                    }
                }

                ui.separator();
                // 3. Remasterización en Alta Definición (Sprites HD)
                ui.heading("✨ Vincular Remasterización HD (PNG / BMP)");
                ui.label("Asocia una ilustración en alta definición a este personaje completo:");
                ui.horizontal(|ui| {
                    if ui.button("📂 Vincular Imagen HD...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Imagen PNG o BMP", &["png", "bmp"])
                            .pick_file()
                        {
                            project.hd_sprites.insert(
                                self.selected_tile,
                                HdSpriteBinding {
                                    tile_index: self.selected_tile,
                                    width: w_px as u32,
                                    height: h_px as u32,
                                    bmp_or_rgba_path: path.display().to_string(),
                                    rgba_pixels: vec![0xFF00E5FF; w_px * h_px],
                                },
                            );
                            self.status_msg = format!("Imagen HD vinculada al personaje #{:03X}", self.selected_tile);
                        }
                    }
                    if project.hd_sprites.contains_key(&self.selected_tile) {
                        if ui.button("🗑 Quitar HD").clicked() {
                            project.hd_sprites.remove(&self.selected_tile);
                            self.status_msg = "Remasterización HD eliminada.".to_string();
                        }
                    }
                });

                if let Some(hd) = project.hd_sprites.get(&self.selected_tile) {
                    ui.colored_label(
                        Color32::from_rgb(0, 220, 255),
                        format!("✔ HD Activo: {}", hd.bmp_or_rgba_path),
                    );
                }
            });
        });
    }
}

// =========================================================================
// FUNCIONES AUXILIARES DE LÓGICA METASPRITE Y DIBUJO
// =========================================================================

fn calc_tile_index(
    base_tile: usize,
    tiles_w: usize,
    tiles_h: usize,
    layout: TileLayout,
    t_col: usize,
    t_row: usize,
    total_tiles: usize,
) -> usize {
    let offset = match layout {
        TileLayout::HorizontalRows => t_row * tiles_w + t_col,
        TileLayout::VerticalColumns => t_col * tiles_h + t_row,
    };
    (base_tile + offset).min(total_tiles.saturating_sub(1))
}

pub fn get_metasprite_pixels(
    project: &NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
) -> Vec<Vec<u8>> {
    let (tw, th, w_px, h_px) = size_mode.dimensions();
    let total_tiles = project.total_tiles();
    let mut out = vec![vec![0u8; w_px]; h_px];

    for t_row in 0..th {
        for t_col in 0..tw {
            let t_idx = calc_tile_index(base_tile, tw, th, layout, t_col, t_row, total_tiles);
            let sub_pixels = project.get_tile_pixels(t_idx);
            for r in 0..8 {
                for c in 0..8 {
                    out[t_row * 8 + r][t_col * 8 + c] = sub_pixels[r][c];
                }
            }
        }
    }
    out
}

pub fn set_metasprite_pixel(
    project: &mut NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
    x: usize,
    y: usize,
    color_idx: u8,
) {
    let (tw, th, w_px, h_px) = size_mode.dimensions();
    if x >= w_px || y >= h_px {
        return;
    }
    let total_tiles = project.total_tiles();
    let t_col = x / 8;
    let t_row = y / 8;
    let sub_x = x % 8;
    let sub_y = y % 8;

    let target_tile = calc_tile_index(base_tile, tw, th, layout, t_col, t_row, total_tiles);
    project.set_tile_pixel(target_tile, sub_y, sub_x, color_idx);
}

pub fn flood_fill_metasprite(
    project: &mut NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
    start_x: usize,
    start_y: usize,
    new_color: u8,
) {
    let (_, _, w_px, h_px) = size_mode.dimensions();
    let mut pixels = get_metasprite_pixels(project, base_tile, size_mode, layout);
    let target_color = pixels[start_y][start_x];
    if target_color == new_color {
        return;
    }

    let mut queue = VecDeque::new();
    queue.push_back((start_x, start_y));

    while let Some((cx, cy)) = queue.pop_front() {
        if cx >= w_px || cy >= h_px || pixels[cy][cx] != target_color {
            continue;
        }
        pixels[cy][cx] = new_color;
        set_metasprite_pixel(project, base_tile, size_mode, layout, cx, cy, new_color);

        if cx > 0 {
            queue.push_back((cx - 1, cy));
        }
        if cx + 1 < w_px {
            queue.push_back((cx + 1, cy));
        }
        if cy > 0 {
            queue.push_back((cx, cy - 1));
        }
        if cy + 1 < h_px {
            queue.push_back((cx, cy + 1));
        }
    }
}

pub fn flip_h_metasprite(
    project: &mut NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
) {
    let (_, _, w_px, h_px) = size_mode.dimensions();
    let pixels = get_metasprite_pixels(project, base_tile, size_mode, layout);
    for r in 0..h_px {
        for c in 0..w_px {
            let flipped_col = pixels[r][w_px - 1 - c];
            set_metasprite_pixel(project, base_tile, size_mode, layout, c, r, flipped_col);
        }
    }
}

pub fn flip_v_metasprite(
    project: &mut NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
) {
    let (_, _, w_px, h_px) = size_mode.dimensions();
    let pixels = get_metasprite_pixels(project, base_tile, size_mode, layout);
    for r in 0..h_px {
        for c in 0..w_px {
            let flipped_row = pixels[h_px - 1 - r][c];
            set_metasprite_pixel(project, base_tile, size_mode, layout, c, r, flipped_row);
        }
    }
}

pub fn clear_metasprite(
    project: &mut NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
) {
    let (_, _, w_px, h_px) = size_mode.dimensions();
    for r in 0..h_px {
        for c in 0..w_px {
            set_metasprite_pixel(project, base_tile, size_mode, layout, c, r, 0);
        }
    }
}

fn draw_metasprite(
    painter: &egui::Painter,
    project: &NgsProject,
    base_tile: usize,
    size_mode: SpriteSizeMode,
    layout: TileLayout,
    origin: Pos2,
    px_size: f32,
    pal: &[u8; 4],
) {
    let (tw, th, _, _) = size_mode.dimensions();
    let total_tiles = project.total_tiles();
    for t_row in 0..th {
        for t_col in 0..tw {
            let t_idx = calc_tile_index(base_tile, tw, th, layout, t_col, t_row, total_tiles);
            let sub_origin = Pos2::new(
                origin.x + t_col as f32 * 8.0 * px_size,
                origin.y + t_row as f32 * 8.0 * px_size,
            );
            draw_tile_8x8(painter, project, t_idx, sub_origin, px_size, pal);
        }
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
