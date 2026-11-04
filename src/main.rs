mod codegen;
mod editors;
mod parser;
mod project;
mod runtime;
#[cfg(target_os = "windows")]
mod win32_show;

const WINDOW_TITLE: &str = "NesGameStudio — IDE Visual de Remasterización HD, Audio & Creación 2D/2.5D";

use eframe::egui::{self, Color32};
use editors::audio::AudioEditorState;
use editors::graphics::GraphicsEditorState;
use project::NgsProject;
use runtime::jukebox_apu::JukeboxEngine;
use std::path::Path;

#[derive(PartialEq, Eq, Clone, Copy)]
enum ActiveTab {
    GraphicsGallery,
    AudioJukebox,
    SubroutinesInspector,
}

struct NesGameStudioApp {
    project: NgsProject,
    active_tab: ActiveTab,
    graphics_editor: GraphicsEditorState,
    audio_editor: AudioEditorState,
    jukebox: JukeboxEngine,
    subroutine_filter: String,
    status_bar_msg: String,
}

impl Default for NesGameStudioApp {
    fn default() -> Self {
        let mut app = Self {
            project: NgsProject::default(),
            active_tab: ActiveTab::GraphicsGallery,
            graphics_editor: GraphicsEditorState::default(),
            audio_editor: AudioEditorState::default(),
            jukebox: JukeboxEngine::new(),
            subroutine_filter: "TODAS".to_string(),
            status_bar_msg: "Listo. Abre un archivo .C exportado por NesEmuRust para explorar sus gráficos y música.".to_string(),
        };

        // Si existe un archivo .c en ../NesEmuRust/exports/, cargarlo automáticamente al iniciar
        let default_contra = Path::new(r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\exports\Contra _USA_.c");
        if default_contra.exists() {
            if let Ok(msg) = parser::load_c_and_asm_into_project(default_contra, &mut app.project) {
                app.status_bar_msg = format!("Auto-cargado: {}", msg);
            }
        }
        app
    }
}

impl eframe::App for NesGameStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Redibujo continuo si la rockola está reproduciendo para animar el osciloscopio
        if let Ok(st) = self.jukebox.state.lock() {
            if st.playing {
                ctx.request_repaint();
            }
        }

        // BARRA SUPERIOR DE HERRAMIENTAS
        egui::TopBottomPanel::top("top_toolbar").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("🎮 NesGameStudio v0.1 (Fase 1)");
                ui.separator();

                if ui.button("📂 Abrir Juego (.C / .ASM)").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Código C de NesEmuRust", &["c"])
                        .pick_file()
                    {
                        match parser::load_c_and_asm_into_project(&path, &mut self.project) {
                            Ok(msg) => {
                                if let Ok(mut st) = self.jukebox.state.lock() {
                                    st.rom_seed.clear();
                                }
                                self.status_bar_msg = msg;
                            }
                            Err(e) => self.status_bar_msg = e,
                        }
                    }
                }

                if ui.button("💾 Guardar Proyecto (.ngs.json)").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name(&format!("{}.ngs.json", self.project.game_title))
                        .add_filter("Proyecto NesGameStudio", &["json"])
                        .save_file()
                    {
                        match self.project.save_project_manifest(&path) {
                            Ok(_) => self.status_bar_msg = format!("Proyecto guardado en {}", path.display()),
                            Err(e) => self.status_bar_msg = e,
                        }
                    }
                }

                ui.separator();

                if ui.button("🔨 Generar .C Modificado").clicked() {
                    match codegen::generate_modified_c_file(&self.project) {
                        Ok(p) => self.status_bar_msg = format!("Código C inyectado en: {}", p.display()),
                        Err(e) => self.status_bar_msg = e,
                    }
                }

                let play_btn = egui::Button::new(
                    egui::RichText::new("🚀 Compilar y Jugar .EXE (x64)")
                        .strong()
                        .color(Color32::BLACK),
                )
                .fill(Color32::from_rgb(80, 230, 130));

                if ui.add(play_btn).clicked() {
                    match codegen::compile_and_run_modified_c(&self.project, true) {
                        Ok(msg) => self.status_bar_msg = msg,
                        Err(e) => self.status_bar_msg = e,
                    }
                }
            });

            ui.separator();
            // PESTAÑAS DE MÓDULOS DE LA FASE 1
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::GraphicsGallery,
                    "🎨 1. Galería Total de Gráficos & Sprites HD",
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::AudioJukebox,
                    "🎵 2. Rockola Aislada (Sound Test) & .MP3/.WAV",
                );
                ui.selectable_value(
                    &mut self.active_tab,
                    ActiveTab::SubroutinesInspector,
                    "🔍 3. Radiografía de Subrutinas (.ASM <-> .C)",
                );
            });
        });

        // BARRA DE ESTADO INFERIOR
        egui::TopBottomPanel::bottom("bottom_status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Juego: {} | Mapper: {} | PRG: {} KB | Tiles: {} | Subrutinas: {}",
                    self.project.game_title,
                    self.project.mapper_id,
                    self.project.prg_rom.len() / 1024,
                    self.project.total_tiles(),
                    self.project.subroutines.len()
                ));
                ui.separator();
                ui.colored_label(Color32::from_rgb(130, 220, 255), &self.status_bar_msg);
            });
        });

        // PANEL CENTRAL SEGÚN PESTAÑA ACTIVA
        egui::CentralPanel::default().show(ctx, |ui| match self.active_tab {
            ActiveTab::GraphicsGallery => {
                self.graphics_editor.ui(ui, &mut self.project);
            }
            ActiveTab::AudioJukebox => {
                self.audio_editor.ui(ui, &mut self.project, &self.jukebox);
            }
            ActiveTab::SubroutinesInspector => {
                self.render_subroutines_tab(ui);
            }
        });
    }
}

impl NesGameStudioApp {
    fn render_subroutines_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 Explorador de Subrutinas y Vectores (.ASM <-> .C)");
        ui.horizontal(|ui| {
            ui.label(format!(
                "Vectores del Juego: NMI=${:04X} | RESET=${:04X} | IRQ=${:04X}",
                self.project.vec_nmi, self.project.vec_reset, self.project.vec_irq
            ));
            ui.separator();
            ui.label("Filtrar por Categoría:");
            for cat in ["TODAS", "[AUDIO / APU]", "[VIDEO / PPU]", "[JOYPAD / INPUT]", "[GAME LOGIC]"] {
                if ui.selectable_label(self.subroutine_filter == cat, cat).clicked() {
                    self.subroutine_filter = cat.to_string();
                }
            }
        });
        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for sub in &self.project.subroutines {
                if self.subroutine_filter != "TODAS" && sub.category != self.subroutine_filter {
                    continue;
                }
                let header = format!(
                    "{} — {} (${:04X}) [Línea {}]",
                    sub.category, sub.name, sub.addr, sub.line_in_asm
                );
                ui.collapsing(header, |ui| {
                    for line in &sub.preview_asm {
                        ui.monospace(line);
                    }
                });
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(target_os = "windows")]
    win32_show::ensure_on_default_desktop();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(WINDOW_TITLE)
            .with_inner_size([1180.0, 760.0])
            .with_visible(true)
            .with_active(true),
        ..Default::default()
    };
    eframe::run_native(
        WINDOW_TITLE,
        options,
        Box::new(|cc| {
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            #[cfg(target_os = "windows")]
            win32_show::force_show_window_after_creation(WINDOW_TITLE);
            Ok(Box::new(NesGameStudioApp::default()))
        }),
    )
}
