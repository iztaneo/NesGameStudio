use eframe::egui::{self, Color32, RichText};

/// Pantalla de bienvenida con pasos claros cuando no hay proyecto abierto.
pub fn welcome_ui(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.add_space(30.0);
        ui.heading(RichText::new("Bienvenido a NesGameStudio").size(26.0));
        ui.label("Explora, edita y remasteriza los gráficos y la música de un juego de NES.");
        ui.add_space(20.0);
    });

    egui::Frame::group(ui.style()).inner_margin(16.0).show(ui, |ui| {
        ui.label(RichText::new("¿Cómo empiezo?").strong().size(18.0));
        ui.add_space(6.0);
        step(ui, "1", "Abre un juego", "Menú  Archivo → Abrir ROM .NES...  (ej. rooms\\Contra (USA).nes)\n\
            o  Archivo → Abrir Código .C...  (ej. NesEmuRust\\exports\\Contra _USA_.c).");
        step(ui, "2", "Espera unos segundos", "El estudio ejecuta el juego por dentro para que él mismo \
            descomprima sus gráficos y arranque su música.");
        step(ui, "3", "Explora los Gráficos", "Menú  Vista → Gráficos. Elige una escena, haz clic en un \
            personaje y edita sus píxeles.");
        step(ui, "4", "Escucha la Música", "Menú  Vista → Rockola. Haz clic en una escena para oír su \
            canción y usa Mute / Solo para aislar instrumentos.");
        step(ui, "5", "Genera tu versión", "Menú  Compilar → Compilar y Ejecutar .EXE  para jugar con tus cambios.");
    });
}

fn step(ui: &mut egui::Ui, num: &str, title: &str, body: &str) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(" {} ", num))
                .strong()
                .size(18.0)
                .color(Color32::BLACK)
                .background_color(Color32::from_rgb(90, 200, 255)),
        );
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong());
            ui.label(body);
        });
    });
    ui.add_space(8.0);
}

/// Franja de ayuda breve que aparece arriba de cada vista.
pub fn tab_hint(ui: &mut egui::Ui, text: &str) {
    egui::Frame::none()
        .fill(Color32::from_rgb(28, 40, 56))
        .inner_margin(8.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.label(RichText::new(format!("💡 {}", text)).color(Color32::from_rgb(190, 225, 255)));
        });
    ui.add_space(6.0);
}
