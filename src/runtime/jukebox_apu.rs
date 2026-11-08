use crate::project::{
    AudioEventKey, AudioEventKind, AudioTrigger, AudioValidationStatus, DiscoveredTrack,
    NgsProject, SoundTrigger,
};
use nes_emu_rust::nes::Nes;
use nes_emu_rust::ui::audio::AudioOutput;
use std::fs::File;
use std::io::Write;
use std::path::Path;

fn make_track(
    trigger: AudioTrigger,
    id: u8,
    kind: AudioEventKind,
    status: AudioValidationStatus,
    title: impl Into<String>,
) -> DiscoveredTrack {
    DiscoveredTrack::with_key(AudioEventKey::new(trigger, id), kind, status, title)
}

/// Ejecuta de forma segura una subrutina del juego con el acumulador A cargado con sound_id
pub fn call_subroutine_with_a(nes: &mut Nes, sub_addr: u16, a_val: u8) -> bool {
    let mut guard = 0;
    while (nes.bus.ppu.scanline < 30 || nes.bus.ppu.scanline > 150) && guard < 20000 {
        nes.step_instruction();
        guard += 1;
    }
    let orig_pc = nes.cpu.pc;
    let orig_sp = nes.cpu.sp;
    let orig_a = nes.cpu.a;
    let orig_x = nes.cpu.x;
    let orig_y = nes.cpu.y;
    let orig_status = nes.cpu.status;

    // Simular JSR sub_addr
    let ret_minus_1 = orig_pc.wrapping_sub(1);
    nes.bus.cpu_write(0x0100 | (nes.cpu.sp as u16), (ret_minus_1 >> 8) as u8);
    nes.cpu.sp = nes.cpu.sp.wrapping_sub(1);
    nes.bus.cpu_write(0x0100 | (nes.cpu.sp as u16), (ret_minus_1 & 0xFF) as u8);
    nes.cpu.sp = nes.cpu.sp.wrapping_sub(1);

    nes.cpu.a = a_val;
    nes.cpu.status.set(nes_emu_rust::cpu::StatusFlags::ZERO, a_val == 0);
    nes.cpu.status.set(nes_emu_rust::cpu::StatusFlags::NEGATIVE, (a_val & 0x80) != 0);
    nes.cpu.pc = sub_addr;

    let mut steps = 0;
    while nes.cpu.pc != orig_pc && steps < 5000 {
        nes.step_instruction();
        steps += 1;
    }

    nes.cpu.pc = orig_pc;
    nes.cpu.sp = orig_sp;
    nes.cpu.a = orig_a;
    nes.cpu.x = orig_x;
    nes.cpu.y = orig_y;
    nes.cpu.status = orig_status;
    steps < 5000
}

/// Detecta el motor de sonido y las pistas reales del juego cargado:
/// primero verifica títulos populares curados, y para cualquier otro juego realiza autodescubrimiento universal
pub fn detect_or_discover_audio_engine(project: &mut NgsProject) {
    project.discovered_tracks.clear();
    let title_lower = project.game_title.to_lowercase();

    // 1. Detección curada para títulos populares
    if title_lower.contains("contra") {
        let trig = AudioTrigger::Subroutine(0xC16B);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        project.discovered_tracks = vec![
            make_track(trig.clone(), 0x26, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 26 — Pantalla de Título & Menú"),
            make_track(trig.clone(), 0x2A, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 2A — Nivel 1: Selva / Jungla"),
            make_track(trig.clone(), 0x3E, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 3E — Nivel 2: Base Militar 3D"),
            make_track(trig.clone(), 0x2E, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 2E — Nivel 3: Cascada"),
            make_track(trig.clone(), 0x32, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 32 — Nivel 5: Campo de Nieve"),
            make_track(trig.clone(), 0x36, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 36 — Nivel 6: Zona Energética"),
            make_track(trig.clone(), 0x3A, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 3A — Nivel 7: Hangar"),
            make_track(trig.clone(), 0x42, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 42 — Nivel 8: Guarida Alienígena"),
            make_track(trig.clone(), 0x46, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 46 — Batalla de Jefe (Boss Fight)"),
            make_track(trig.clone(), 0x4E, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Pista 4E — Nivel Completado (Fanfarria)"),
            make_track(trig.clone(), 0x52, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 52 — Final y Créditos"),
            make_track(trig.clone(), 0x54, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Pista 54 — Game Over"),
            make_track(trig.clone(), 0x10, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 10 — Disparo de Rifle Normal"),
            make_track(trig.clone(), 0x11, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 11 — Disparo Spread Gun (Escopeta)"),
            make_track(trig.clone(), 0x12, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 12 — Disparo de Rayo Láser"),
            make_track(trig.clone(), 0x13, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 13 — Disparo de Fuego / Lanzallamas"),
            make_track(trig.clone(), 0x15, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 15 — Explosión Pequeña"),
            make_track(trig.clone(), 0x16, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 16 — Explosión Grande / Jefe"),
            make_track(trig.clone(), 0x19, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 19 — Halcón de Arma en Vuelo"),
            make_track(trig.clone(), 0x1A, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 1A — Salto del Soldado"),
            make_track(trig.clone(), 0x1E, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 1E — Daño / Caída de Soldado"),
            make_track(trig.clone(), 0x24, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 24 — Alarma / Sirena de la Base"),
            make_track(trig.clone(), 0x25, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 25 — Rugido de Jefe Alien"),
            make_track(trig.clone(), 0x55, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 55 — Vida Extra (1-UP)"),
        ];
        return;
    }

    if title_lower.contains("mario") {
        let bgm_trig = AudioTrigger::RamAddress(0x04F5);
        let bgm_map_trig = AudioTrigger::RamAddress(0x04F4);
        let sfx_trig_1 = AudioTrigger::RamAddress(0x04F1);
        let sfx_trig_2 = AudioTrigger::RamAddress(0x04F2);
        let sfx_trig_3 = AudioTrigger::RamAddress(0x04F3);
        let sfx_trig_6 = AudioTrigger::RamAddress(0x04F6);
        project.sound_trigger = bgm_trig.clone();
        project.register_audio_trigger(bgm_trig.clone());
        project.register_audio_trigger(bgm_map_trig.clone());
        project.register_audio_trigger(sfx_trig_1.clone());
        project.register_audio_trigger(sfx_trig_2.clone());
        project.register_audio_trigger(sfx_trig_3.clone());
        project.register_audio_trigger(sfx_trig_6.clone());
        project.discovered_tracks = vec![
            // Temas Musicales de Nivel y Batalla ($04F5)
            make_track(bgm_trig.clone(), 0x0D, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 0D — Pantalla de Título ($04F5=$0D)"),
            make_track(bgm_trig.clone(), 0x01, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 01 — Nivel Superficie / Overworld ($04F5=$01)"),
            make_track(bgm_trig.clone(), 0x02, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 02 — Nivel Subterráneo / Underground ($04F5=$02)"),
            make_track(bgm_trig.clone(), 0x03, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 03 — Nivel Acuático / Underwater ($04F5=$03)"),
            make_track(bgm_trig.clone(), 0x04, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 04 — Fortaleza / Castillo ($04F5=$04)"),
            make_track(bgm_trig.clone(), 0x05, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 05 — Barco Volador / Airship ($04F5=$05)"),
            make_track(bgm_trig.clone(), 0x06, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 06 — Estrella de Invencibilidad / Starman ($04F5=$06)"),
            make_track(bgm_trig.clone(), 0x07, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 07 — Batalla de Jefe Koopaling ($04F5=$07)"),
            make_track(bgm_trig.clone(), 0x0E, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 0E — Casa de Toad / Minijuego ($04F5=$0E)"),
            make_track(bgm_trig.clone(), 0x0F, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 0F — Batalla Final contra Bowser ($04F5=$0F)"),
            // Temas Musicales de los Mapas de Mundo ($04F4)
            make_track(bgm_map_trig.clone(), 0x01, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 01 — Mundo 1: Grass Land ($04F4=$01)"),
            make_track(bgm_map_trig.clone(), 0x02, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 02 — Mundo 2: Desert Land ($04F4=$02)"),
            make_track(bgm_map_trig.clone(), 0x04, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 03 — Mundo 3: Water Land ($04F4=$04)"),
            make_track(bgm_map_trig.clone(), 0x08, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 04 — Mundo 4: Giant Land ($04F4=$08)"),
            make_track(bgm_map_trig.clone(), 0x10, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 05 — Mundo 5: Sky Land ($04F4=$10)"),
            make_track(bgm_map_trig.clone(), 0x20, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 06 — Mundo 6: Ice Land ($04F4=$20)"),
            make_track(bgm_map_trig.clone(), 0x40, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Mapa 07 — Mundo 7: Pipe Land ($04F4=$40)"),
            // Fanfarrias ($04F5)
            make_track(bgm_trig.clone(), 0x08, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Fanfarria 08 — Nivel Completado ($04F5=$08)"),
            make_track(bgm_trig.clone(), 0x0C, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Fanfarria 0C — Game Over ($04F5=$0C)"),
            // Comando de Control ($04F4=$80)
            make_track(bgm_map_trig.clone(), 0x80, AudioEventKind::Control, AudioValidationStatus::Confirmed, "Control $80 — Detener Música / Silencio ($04F4=$80)"),
            // Efectos de Sonido: Cola 1 ($04F1 - Movimiento y Mario)
            make_track(sfx_trig_1.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$01 — Salto de Mario"),
            make_track(sfx_trig_1.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$02 — Golpe a Bloque / Rebote"),
            make_track(sfx_trig_1.clone(), 0x04, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$04 — Derrape / Deslizamiento"),
            make_track(sfx_trig_1.clone(), 0x08, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$08 — Patada a Caparazón"),
            make_track(sfx_trig_1.clone(), 0x10, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$10 — Entrada por Tubería / Encoger"),
            make_track(sfx_trig_1.clone(), 0x20, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$20 — Bola de Fuego"),
            make_track(sfx_trig_1.clone(), 0x80, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F1:$80 — Latigazo de Cola Mapache"),
            // Efectos de Sonido: Cola 2 ($04F2 - Ítems, Monedas y Power-Ups)
            make_track(sfx_trig_2.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$01 — Moneda"),
            make_track(sfx_trig_2.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$02 — Aparece Champiñón / Flor"),
            make_track(sfx_trig_2.clone(), 0x04, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$04 — Aparece Enredadera"),
            make_track(sfx_trig_2.clone(), 0x08, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$08 — Disparo de Cañón / Explosión"),
            make_track(sfx_trig_2.clone(), 0x10, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$10 — Tic-Tac / Cursor de Mapa"),
            make_track(sfx_trig_2.clone(), 0x20, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$20 — Crecer / Power-Up"),
            make_track(sfx_trig_2.clone(), 0x40, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$40 — Vida Extra (1-UP)"),
            make_track(sfx_trig_2.clone(), 0x80, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F2:$80 — Transformación Puff / Traje"),
            // Efectos de Sonido: Cola 3 ($04F3 - Ambiente y Ruido)
            make_track(sfx_trig_3.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F3:$01 — Romper Bloque de Ladrillo"),
            make_track(sfx_trig_3.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F3:$02 — Llamarada de Bowser / Fuego"),
            make_track(sfx_trig_3.clone(), 0x08, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F3:$08 — Paso de Barco Volador / Trueno"),
            // Efectos de Sonido: Cola 4 ($04F6 - Interfaz y Pausa)
            make_track(sfx_trig_6.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F6:$01 — Mover en Mapa del Mundo"),
            make_track(sfx_trig_6.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F6:$02 — Abrir Inventario de Ítems"),
            make_track(sfx_trig_6.clone(), 0x80, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX $04F6:$80 — Sonido de Pausa"),
        ];
        return;
    }

    if title_lower.contains("castlevania") || title_lower.contains("akumajou") {
        let sub = if title_lower.contains("akumajou") { 0xE249 } else { 0xE25F };
        let trig = AudioTrigger::Subroutine(sub);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        project.discovered_tracks = vec![
            make_track(trig.clone(), 0x4B, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 4B — Prelude (Pantalla de Título)"),
            make_track(trig.clone(), 0x63, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 63 — Beginning (Nivel 1)"),
            make_track(trig.clone(), 0x09, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 09 — Clockwork (Torre del Reloj)"),
            make_track(trig.clone(), 0x0C, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 0C — Mad Forest (Bosque Maldito)"),
            make_track(trig.clone(), 0x46, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 46 — Stream (Río Subterráneo)"),
            make_track(trig.clone(), 0x3A, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 3A — Aquarius (Templo Sumergido)"),
            make_track(trig.clone(), 0x6C, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 6C — Anxiety (Cueva)"),
            make_track(trig.clone(), 0x3D, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 3D — Rising (Castillo Exterior)"),
            make_track(trig.clone(), 0x3B, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 3B — Dead Beat (Pantano)"),
            make_track(trig.clone(), 0x35, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 35 — Boss Fight (Batalla de Jefe)"),
            make_track(trig.clone(), 0x2C, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Pista 2C — Fanfarria de Victoria"),
            make_track(trig.clone(), 0x6B, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Pista 6B — Game Over"),
            make_track(trig.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 01 — Latigazo"),
            make_track(trig.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 02 — Salto"),
            make_track(trig.clone(), 0x03, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 03 — Recoger Corazón"),
            make_track(trig.clone(), 0x04, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 04 — Golpe / Daño"),
        ];
        return;
    }

    if title_lower.contains("batman") {
        let trig = AudioTrigger::RamAddress(0x0042);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        let bgm_ids = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Music,
                AudioValidationStatus::Confirmed,
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
            ));
        }
        let sfx_ids = [0x20u8, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Sfx,
                AudioValidationStatus::Confirmed,
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
            ));
        }
        return;
    }

    if title_lower.contains("ninja") {
        let trig = AudioTrigger::RamAddress(0x0700);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        let bgm_ids = [0x04u8, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x1E, 0x2A, 0x33];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Music,
                AudioValidationStatus::Confirmed,
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
            ));
        }
        let sfx_ids = [0x0Eu8, 0x0F, 0x10, 0x19, 0x1A, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Sfx,
                AudioValidationStatus::Confirmed,
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
            ));
        }
        return;
    }

    if title_lower.contains("chip") || title_lower.contains("dale") {
        let trig = AudioTrigger::RamAddress(0x00F0);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        let bgm_ids = [0x72u8, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Music,
                AudioValidationStatus::Confirmed,
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
            ));
        }
        let sfx_ids = [0x22u8, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Sfx,
                AudioValidationStatus::Confirmed,
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
            ));
        }
        return;
    }

    if title_lower.contains("tiny toon") {
        let trig = AudioTrigger::Subroutine(0xF823);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        let bgm_ids = [0x28u8, 0x5E, 0x62, 0x6A, 0x6C];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Music,
                AudioValidationStatus::Confirmed,
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
            ));
        }
        let sfx_ids = [0x02u8, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                trig.clone(),
                id,
                AudioEventKind::Sfx,
                AudioValidationStatus::Confirmed,
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
            ));
        }
        return;
    }

    if title_lower.contains("zelda") {
        let bgm_trig = AudioTrigger::RamAddress(0x0600);
        let sfx_trig_4 = AudioTrigger::RamAddress(0x0604);
        let sfx_trig_2 = AudioTrigger::RamAddress(0x0602);
        project.sound_trigger = bgm_trig.clone();
        project.register_audio_trigger(bgm_trig.clone());
        project.register_audio_trigger(sfx_trig_4.clone());
        project.register_audio_trigger(sfx_trig_2.clone());
        project.discovered_tracks = vec![
            make_track(bgm_trig.clone(), 0x01, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Melodía 01 — Tema de Superficie / Hyrule (ID $01)"),
            make_track(bgm_trig.clone(), 0x02, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Melodía 02 — Mazmorra / Inframundo (ID $02)"),
            make_track(bgm_trig.clone(), 0x04, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Melodía 03 — Mazmorra Final / Death Mountain (ID $04)"),
            make_track(bgm_trig.clone(), 0x10, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Melodía 04 — Introducción / Pantalla de Título (ID $10)"),
            make_track(bgm_trig.clone(), 0x20, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Melodía 05 — Game Over / Continuar (ID $20)"),
            make_track(bgm_trig.clone(), 0x40, AudioEventKind::Fanfare, AudioValidationStatus::Confirmed, "Melodía 06 — Fanfarria de Trifuerza (ID $40)"),
            make_track(bgm_trig.clone(), 0x80, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Melodía 07 — Rescate de Zelda y Créditos (ID $80)"),
            make_track(sfx_trig_4.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 01 — Espada / Ataque ($0604=$01)"),
            make_track(sfx_trig_4.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 02 — Bumerán / Flecha ($0604=$02)"),
            make_track(sfx_trig_4.clone(), 0x04, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 03 — Flauta / Magia ($0604=$04)"),
            make_track(sfx_trig_2.clone(), 0x01, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 04 — Secreto Descubierto ($0602=$01)"),
            make_track(sfx_trig_2.clone(), 0x02, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 05 — Explosión de Bomba ($0602=$02)"),
            make_track(sfx_trig_2.clone(), 0x04, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 06 — Golpe / Daño a Enemigo ($0602=$04)"),
            make_track(sfx_trig_2.clone(), 0x08, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 07 — Recoger Corazón / Rupia ($0602=$08)"),
        ];
        return;
    }

    if title_lower.contains("simpson") {
        let bgm_trig = AudioTrigger::Subroutine(0xFAC7);
        let sfx_trig = AudioTrigger::Subroutine(0xFADA);
        project.sound_trigger = bgm_trig.clone();
        project.register_audio_trigger(bgm_trig.clone());
        project.register_audio_trigger(sfx_trig.clone());
        for id in 0x01..=0x0Du8 {
            project.discovered_tracks.push(make_track(
                bgm_trig.clone(),
                id,
                AudioEventKind::Music,
                AudioValidationStatus::Confirmed,
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", id, id),
            ));
        }
        for sfx_num in 0x01..=0x0Cu8 {
            project.discovered_tracks.push(make_track(
                sfx_trig.clone(),
                sfx_num,
                AudioEventKind::Sfx,
                AudioValidationStatus::Confirmed,
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", sfx_num, sfx_num),
            ));
        }
        return;
    }

    if title_lower.contains("kirby") {
        let bgm_trig = AudioTrigger::Subroutine(0xF862);
        let sfx_trig = AudioTrigger::Subroutine(0xF826);
        project.sound_trigger = bgm_trig.clone();
        project.register_audio_trigger(bgm_trig.clone());
        project.register_audio_trigger(sfx_trig.clone());
        let bgm_ids = [0x01u8, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x14, 0x1C, 0x20, 0x21, 0x23];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(make_track(
                bgm_trig.clone(),
                id,
                AudioEventKind::Music,
                AudioValidationStatus::Confirmed,
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
            ));
        }
        for sfx_num in 0x01..=0x0Cu8 {
            project.discovered_tracks.push(make_track(
                sfx_trig.clone(),
                sfx_num,
                AudioEventKind::Sfx,
                AudioValidationStatus::Confirmed,
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", sfx_num, sfx_num),
            ));
        }
        return;
    }

    if title_lower.contains("battletoads") {
        let trig = AudioTrigger::Subroutine(0x9148);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        project.discovered_tracks = vec![
            make_track(trig.clone(), 0x01, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 01 — Pantalla de Título (Battletoads Theme)"),
            make_track(trig.clone(), 0x02, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 02 — Ragnarok's Canyon (Nivel 1)"),
            make_track(trig.clone(), 0x03, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 03 — Wookie Hole (Nivel 2)"),
            make_track(trig.clone(), 0x04, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 04 — Turbo Tunnel (Nivel 3)"),
            make_track(trig.clone(), 0x05, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 05 — Arctic Caverns (Nivel 4)"),
            make_track(trig.clone(), 0x06, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 06 — Surf City (Nivel 5)"),
            make_track(trig.clone(), 0x07, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 07 — Karnath's Lair (Nivel 6)"),
            make_track(trig.clone(), 0x08, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 08 — Volkmire's Inferno (Nivel 7)"),
            make_track(trig.clone(), 0x09, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 09 — Intruder Excluder (Nivel 8)"),
            make_track(trig.clone(), 0x0A, AudioEventKind::Music, AudioValidationStatus::Confirmed, "Pista 0A — Batalla contra Dark Queen"),
            make_track(trig.clone(), 0x10, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 10 — Puñetazo Normal"),
            make_track(trig.clone(), 0x11, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 11 — Super Golpe / Bota Gigante"),
            make_track(trig.clone(), 0x12, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 12 — Salto"),
            make_track(trig.clone(), 0x13, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 13 — Impacto / Explosión"),
            make_track(trig.clone(), 0x14, AudioEventKind::Sfx, AudioValidationStatus::Confirmed, "SFX 14 — Pausa (Beatbox)"),
        ];
        return;
    }

    // 2. Autodescubrimiento dinámico universal para cualquier otro juego
    auto_discover_sound_engine_dynamically(project);
}

fn auto_discover_sound_engine_dynamically(project: &mut NgsProject) {
    use std::collections::HashMap;
    let prg = &project.prg_rom;
    if prg.len() < 5 {
        project.sound_trigger = SoundTrigger::Unknown;
        fallback_discovered_tracks(project);
        return;
    }

    let mut jsr_counts: HashMap<u16, Vec<u8>> = HashMap::new();
    let mut sta_counts: HashMap<u16, Vec<u8>> = HashMap::new();

    for i in 0..prg.len().saturating_sub(5) {
        if prg[i] == 0xA9 {
            let imm = prg[i + 1];
            if imm == 0 || imm == 0xFF {
                continue;
            }
            if prg[i + 2] == 0x20 || prg[i + 2] == 0x4C {
                let addr = u16::from_le_bytes([prg[i + 3], prg[i + 4]]);
                if addr >= 0x8000 {
                    let list = jsr_counts.entry(addr).or_default();
                    if !list.contains(&imm) {
                        list.push(imm);
                    }
                }
            } else if prg[i + 2] == 0x85 {
                let addr = prg[i + 3] as u16;
                let list = sta_counts.entry(addr).or_default();
                if !list.contains(&imm) {
                    list.push(imm);
                }
            } else if prg[i + 2] == 0x8D {
                let addr = u16::from_le_bytes([prg[i + 3], prg[i + 4]]);
                if addr < 0x0800 {
                    let list = sta_counts.entry(addr).or_default();
                    if !list.contains(&imm) {
                        list.push(imm);
                    }
                }
            }
        }
    }

    let mut top_jsr: Vec<(u16, Vec<u8>)> = jsr_counts.into_iter().filter(|(_, v)| v.len() >= 4).collect();
    top_jsr.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    let mut top_sta: Vec<(u16, Vec<u8>)> = sta_counts.into_iter().filter(|(_, v)| v.len() >= 4).collect();
    top_sta.sort_by_key(|(_, v)| std::cmp::Reverse(v.len()));

    // Si tenemos estado base, probar los candidatos más probables y conservar MÚLTIPLES rutas válidas (Fase 3)
    if let Some(base_state) = project.base_audio_save_state.clone() {
        let mut accepted_triggers: Vec<(AudioTrigger, Vec<u8>)> = Vec::new();

        // Probar candidatos de subrutina (JSR $XXXX)
        for (sub, imms) in top_jsr.iter().take(5) {
            let mut distinct_sigs = Vec::new();
            for &test_id in imms.iter().take(5) {
                let _ = project.nes_live.from_save_state_bytes(&base_state);
                if call_subroutine_with_a(&mut project.nes_live, *sub, test_id) {
                    let mut sig = Vec::new();
                    for _ in 0..15 {
                        project.nes_live.step_frame();
                        sig.push((
                            project.nes_live.bus.apu.pulse1.timer_period,
                            project.nes_live.bus.apu.pulse2.timer_period,
                            project.nes_live.bus.apu.triangle.timer_period,
                        ));
                    }
                    if !distinct_sigs.contains(&sig) {
                        distinct_sigs.push(sig);
                    }
                }
            }
            if distinct_sigs.len() >= 2 {
                let trig = AudioTrigger::Subroutine(*sub);
                project.register_audio_trigger(trig.clone());
                accepted_triggers.push((trig, imms.clone()));
                if accepted_triggers.len() >= 3 {
                    break;
                }
            }
        }

        // Probar candidatos de escritura en RAM (STA $XXXX)
        for (addr, imms) in top_sta.iter().take(5) {
            if accepted_triggers.len() >= 4 {
                break;
            }
            let mut distinct_sigs = Vec::new();
            for &test_id in imms.iter().take(5) {
                let _ = project.nes_live.from_save_state_bytes(&base_state);
                project.nes_live.bus.cpu_ram[(*addr as usize) & 0x7FF] = test_id;
                let mut sig = Vec::new();
                for _ in 0..15 {
                    project.nes_live.step_frame();
                    sig.push((
                        project.nes_live.bus.apu.pulse1.timer_period,
                        project.nes_live.bus.apu.pulse2.timer_period,
                        project.nes_live.bus.apu.triangle.timer_period,
                    ));
                }
                if !distinct_sigs.contains(&sig) {
                    distinct_sigs.push(sig);
                }
            }
            if distinct_sigs.len() >= 2 {
                let trig = AudioTrigger::RamAddress(*addr);
                project.register_audio_trigger(trig.clone());
                accepted_triggers.push((trig, imms.clone()));
            }
        }

        if !accepted_triggers.is_empty() {
            project.sound_trigger = accepted_triggers[0].0.clone();
            for (idx, (trig, imms)) in accepted_triggers.iter().enumerate() {
                classify_tracks_for_trigger(project, trig.clone(), imms, idx == 0);
            }
            if project.discovered_tracks.is_empty() {
                fallback_discovered_tracks(project);
            }
            return;
        }
    }

    if let Some((sub, imms)) = top_jsr.first() {
        let trig = AudioTrigger::Subroutine(*sub);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        classify_tracks_for_trigger(project, trig, imms, true);
    } else if let Some((addr, imms)) = top_sta.first() {
        let trig = AudioTrigger::RamAddress(*addr);
        project.sound_trigger = trig.clone();
        project.register_audio_trigger(trig.clone());
        classify_tracks_for_trigger(project, trig, imms, true);
    } else {
        project.sound_trigger = SoundTrigger::Unknown;
        fallback_discovered_tracks(project);
    }
}

/// Clasifica dinámicamente las pistas de un `AudioTrigger` comparando contra la línea base del estado guardado (Fase 4):
/// 1. Graba la firma APU de referencia (`base_early`, `base_late`) sin disparar ningún ID.
/// 2. Para cada candidato, restaura el estado aislado, mide energía RMS normalizada, duración activa en frames
///    y actividad por canal (`Pulse 1`, `Pulse 2`, `Triangle`, `Noise`, `DMC`).
/// 3. Distingue `Music` (BGM continuo > 115 frames), `Fanfare` (melodía de 50..115 frames que concluye),
///    `Sfx` (efecto corto o de un solo canal) y `Control` (detiene la música activa de la línea base).
fn classify_tracks_for_trigger(
    project: &mut NgsProject,
    trigger: AudioTrigger,
    extra_imms: &[u8],
    scan_full_range: bool,
) {
    let Some(base_state) = project.base_audio_save_state.clone() else {
        fallback_discovered_tracks(project);
        return;
    };

    let mut candidates: Vec<u8> = if scan_full_range {
        (1..=0x58u8).collect()
    } else {
        Vec::new()
    };
    for &imm in extra_imms {
        if imm > 0 && imm != 0xFF && !candidates.contains(&imm) {
            candidates.push(imm);
        }
    }
    candidates.sort();

    project.nes_live.bus.apu.set_sample_rate(44100);
    let mut samples_early = Vec::with_capacity(2048);
    let mut samples_late = Vec::with_capacity(2048);

    // 1. Medir línea base sin disparar ningún sonido (por si la pantalla de título ya tiene música sonando)
    let _ = project.nes_live.from_save_state_bytes(&base_state);
    let mut base_early_sigs = Vec::with_capacity(18);
    let mut base_late_sigs = Vec::with_capacity(27);
    let mut base_late_samples = Vec::with_capacity(2048);
    let mut dummy_samples = Vec::with_capacity(2048);
    for _ in 0..18 {
        project.nes_live.step_frame();
        project.nes_live.take_audio_samples(&mut dummy_samples);
        base_early_sigs.push(apu_frame_sig(&project.nes_live));
    }
    for _ in 0..27 {
        project.nes_live.step_frame();
        project.nes_live.take_audio_samples(&mut base_late_samples);
        base_late_sigs.push(apu_frame_sig(&project.nes_live));
    }
    let base_late_energy: f32 = base_late_samples.iter().map(|s| s.abs()).sum();

    let mut seen_signatures: Vec<(Vec<(u16, u16, u16, u16)>, Vec<(u16, u16, u16, u16)>)> = Vec::new();

    for id in candidates {
        let _ = project.nes_live.from_save_state_bytes(&base_state);
        match trigger {
            SoundTrigger::Subroutine(sub) => {
                if !call_subroutine_with_a(&mut project.nes_live, sub, id) {
                    continue;
                }
            }
            SoundTrigger::RamAddress(addr) => {
                project.nes_live.bus.cpu_ram[(addr as usize) & 0x7FF] = id;
            }
            SoundTrigger::DriverCommand {
                entry_point,
                queue_address,
            } => {
                project.nes_live.bus.cpu_ram[(queue_address as usize) & 0x7FF] = id;
                let _ = call_subroutine_with_a(&mut project.nes_live, entry_point, id);
            }
            SoundTrigger::Unknown => continue,
        }

        let mut active_channels = [false; 5];
        let mut active_frames = 0u16;

        samples_early.clear();
        let mut early_sigs = Vec::with_capacity(18);
        for _ in 0..18 {
            project.nes_live.step_frame();
            project.nes_live.take_audio_samples(&mut samples_early);
            let sig = apu_frame_sig(&project.nes_live);
            early_sigs.push(sig);
            let ch = apu_active_channels_mask(&project.nes_live);
            if ch.iter().any(|&c| c) {
                active_frames += 1;
            }
            for i in 0..5 {
                active_channels[i] |= ch[i];
            }
        }
        let energy_early: f32 = samples_early.iter().map(|s| s.abs()).sum();
        let rms_early: f32 = if samples_early.is_empty() {
            0.0
        } else {
            (samples_early.iter().map(|s| s * s).sum::<f32>() / samples_early.len() as f32).sqrt()
        };

        samples_late.clear();
        let mut late_sigs = Vec::with_capacity(27);
        for _ in 0..27 {
            project.nes_live.step_frame();
            project.nes_live.take_audio_samples(&mut samples_late);
            let sig = apu_frame_sig(&project.nes_live);
            late_sigs.push(sig);
            let ch = apu_active_channels_mask(&project.nes_live);
            if ch.iter().any(|&c| c) {
                active_frames += 1;
            }
            for i in 0..5 {
                active_channels[i] |= ch[i];
            }
        }
        let energy_late: f32 = samples_late.iter().map(|s| s.abs()).sum();

        // Detectar comando de control (silencia música que estaba activa en la línea base)
        if base_late_energy > 400.0 && energy_early < 20.0 && energy_late < 15.0 {
            let key = AudioEventKey::new(trigger.clone(), id);
            let track = DiscoveredTrack::with_key(
                key,
                AudioEventKind::Control,
                AudioValidationStatus::Candidate,
                format!("Control ${:02X} — Silencio / Stop ({})", id, trigger.short_label()),
            )
            .with_metrics(active_frames, active_channels, 0.75);
            project.upsert_discovered_track(track);
            continue;
        }

        // Si no produjo sonido ni cambió nada respecto a la línea base, es un ID vacío/inválido
        if (energy_early <= 35.0 && rms_early < 0.002)
            || (early_sigs == base_early_sigs && late_sigs == base_late_sigs)
        {
            continue;
        }

        // Evitar duplicados exactos (varios IDs que disparan exactamente el mismo sonido)
        let full_sig = (early_sigs.clone(), late_sigs.clone());
        if seen_signatures.contains(&full_sig) {
            continue;
        }
        seen_signatures.push(full_sig);

        // Evaluar cuántos canales melódicos (Pulse1, Pulse2, Triangle) cambiaron y siguen activos en la fase media
        let mut p1_changed_active = 0usize;
        let mut p2_changed_active = 0usize;
        let mut tri_changed_active = 0usize;
        for (cur, base) in late_sigs.iter().zip(base_late_sigs.iter()) {
            if cur.0 != base.0 && cur.0 > 8 {
                p1_changed_active += 1;
            }
            if cur.1 != base.1 && cur.1 > 8 {
                p2_changed_active += 1;
            }
            if cur.2 != base.2 && cur.2 > 8 {
                tri_changed_active += 1;
            }
        }
        let melodic_channels_changed = (p1_changed_active >= 6) as usize
            + (p2_changed_active >= 6) as usize
            + (tri_changed_active >= 6) as usize;

        // Si parece candidato a canción o fanfarria a los 45 frames, verificar hasta el frame 125 (2.1 segundos)
        let mut kind = AudioEventKind::Sfx;
        let mut confidence = 0.72f32;
        if energy_late > 320.0 && melodic_channels_changed >= 1 && late_sigs != base_late_sigs {
            let mut distinct_notes = std::collections::HashSet::new();
            for _ in 45..95 {
                project.nes_live.step_frame();
                project.nes_live.bus.apu.audio_samples.clear();
                let sig = apu_frame_sig(&project.nes_live);
                if sig.0 > 8 { distinct_notes.insert((1u8, sig.0)); }
                if sig.1 > 8 { distinct_notes.insert((2u8, sig.1)); }
                if sig.2 > 8 { distinct_notes.insert((3u8, sig.2)); }
                let ch = apu_active_channels_mask(&project.nes_live);
                if ch.iter().any(|&c| c) {
                    active_frames += 1;
                }
                for i in 0..5 {
                    active_channels[i] |= ch[i];
                }
            }
            let mut sustained_samples = Vec::with_capacity(2048);
            for _ in 95..125 {
                project.nes_live.step_frame();
                project.nes_live.take_audio_samples(&mut sustained_samples);
                let sig = apu_frame_sig(&project.nes_live);
                if sig.0 > 8 { distinct_notes.insert((1u8, sig.0)); }
                if sig.1 > 8 { distinct_notes.insert((2u8, sig.1)); }
                if sig.2 > 8 { distinct_notes.insert((3u8, sig.2)); }
                let ch = apu_active_channels_mask(&project.nes_live);
                if ch.iter().any(|&c| c) {
                    active_frames += 1;
                }
                for i in 0..5 {
                    active_channels[i] |= ch[i];
                }
            }
            let sustained_energy: f32 = sustained_samples.iter().map(|s| s.abs()).sum();
            if sustained_energy > 600.0 && distinct_notes.len() >= 3 {
                kind = AudioEventKind::Music;
                confidence = 0.90;
            } else if melodic_channels_changed >= 2 && distinct_notes.len() >= 4 && active_frames >= 55 {
                kind = AudioEventKind::Fanfare;
                confidence = 0.80;
            }
        }

        let count_so_far = project
            .discovered_tracks
            .iter()
            .filter(|t| t.kind == kind)
            .count()
            + 1;
        let title = match kind {
            AudioEventKind::Music => format!(
                "Melodía {:02} — Tema Musical ({} :: ${:02X})",
                count_so_far,
                trigger.short_label(),
                id
            ),
            AudioEventKind::Fanfare => format!(
                "Fanfarria {:02} — Cortinilla ({} :: ${:02X})",
                count_so_far,
                trigger.short_label(),
                id
            ),
            _ => format!(
                "SFX {:02} — Efecto de Sonido ({} :: ${:02X})",
                count_so_far,
                trigger.short_label(),
                id
            ),
        };
        let track = make_track(
            trigger.clone(),
            id,
            kind,
            AudioValidationStatus::Candidate,
            title,
        )
        .with_metrics(active_frames, active_channels, confidence);
        project.upsert_discovered_track(track);
    }

    let _ = project.nes_live.from_save_state_bytes(&base_state);
}

fn apu_active_channels_mask(nes: &Nes) -> [bool; 5] {
    let (p1, p2, tri, noise) = apu_frame_sig(nes);
    let dmc = nes.bus.apu.dmc.enabled && nes.bus.apu.dmc.bytes_remaining > 0;
    [p1 > 8, p2 > 8, tri > 8, noise > 0, dmc]
}

fn apu_frame_sig(nes: &Nes) -> (u16, u16, u16, u16) {
    let p1 = if nes.bus.apu.pulse1.enabled && nes.bus.apu.pulse1.length_counter > 0 && nes.bus.apu.pulse1.output() > 0 {
        nes.bus.apu.pulse1.timer_period
    } else {
        0
    };
    let p2 = if nes.bus.apu.pulse2.enabled && nes.bus.apu.pulse2.length_counter > 0 && nes.bus.apu.pulse2.output() > 0 {
        nes.bus.apu.pulse2.timer_period
    } else {
        0
    };
    let tri = if nes.bus.apu.triangle.enabled && nes.bus.apu.triangle.length_counter > 0 && nes.bus.apu.triangle.linear_counter > 0 {
        nes.bus.apu.triangle.timer_period
    } else {
        0
    };
    let noise = if nes.bus.apu.noise.enabled && nes.bus.apu.noise.length_counter > 0 && nes.bus.apu.noise.output() > 0 {
        nes.bus.apu.noise.timer_period
    } else {
        0
    };
    (p1, p2, tri, noise)
}

/// Silencia cualquier tema musical de fondo activo en el estado restaurado antes de reproducir un SFX aislado,
/// evitando que un efecto de sonido suene mezclado con la música del mapa o de título (ej. Super Mario Bros. 3 o Zelda).
pub fn silence_active_bgm_for_isolated_sfx(project: &mut NgsProject, sfx_trigger: &AudioTrigger) {
    let title_lower = project.game_title.to_lowercase();
    if title_lower.contains("mario") || matches!(sfx_trigger, AudioTrigger::RamAddress(0x04F1 | 0x04F2 | 0x04F3 | 0x04F6)) {
        // En Super Mario Bros. 3, escribir $80 en $04F4 detiene limpiamente cualquier música BGM activa ($04F4 y $04F5 son las dos colas BGM)
        project.nes_live.bus.cpu_ram[0x04F4] = 0x80;
        project.nes_live.bus.cpu_ram[0x04F5] = 0x00;
        for _ in 0..3 {
            project.nes_live.step_frame();
            project.nes_live.bus.apu.audio_samples.clear();
        }
    } else if title_lower.contains("zelda") || matches!(sfx_trigger, AudioTrigger::RamAddress(0x0602 | 0x0604)) {
        // En The Legend of Zelda, limpiar el estado activo de BGM ($0600, $0605, $060A) y apagar canales tonales previos
        project.nes_live.bus.cpu_ram[0x0600] = 0x00;
        project.nes_live.bus.cpu_ram[0x0605] = 0x00;
        project.nes_live.bus.cpu_ram[0x060A] = 0x00;
        project.nes_live.bus.cpu_write(0x4015, 0x00);
        project.nes_live.bus.cpu_write(0x4015, 0x0F);
    } else if title_lower.contains("contra") {
        // En Contra, el comando $00 o limpiar la APU detiene cualquier melodía activa antes del SFX
        project.nes_live.bus.cpu_write(0x4015, 0x00);
        project.nes_live.bus.cpu_write(0x4015, 0x0F);
    } else {
        // Silenciar registros activos de la APU para aislar el nuevo efecto
        project.nes_live.bus.cpu_write(0x4015, 0x00);
        project.nes_live.bus.cpu_write(0x4015, 0x0F);
    }
    project.nes_live.bus.apu.audio_samples.clear();
}

fn fallback_discovered_tracks(project: &mut NgsProject) {
    let trig = project.sound_trigger.clone();
    for id in 1..=16 {
        project.discovered_tracks.push(make_track(
            trig.clone(),
            id,
            AudioEventKind::Music,
            AudioValidationStatus::Unclassified,
            format!("Pista {:02X} — Tema / Música #{:02X}", id, id),
        ));
    }
    for id in 0x10..=0x25 {
        project.discovered_tracks.push(make_track(
            trig.clone(),
            id,
            AudioEventKind::Sfx,
            AudioValidationStatus::Unclassified,
            format!("SFX {:02X} — Efecto de Sonido #{:02X}", id, id),
        ));
    }
}

#[cfg(target_os = "windows")]
pub mod winmm {
    use std::ffi::CString;

    #[link(name = "winmm")]
    extern "system" {
        fn mciSendStringA(
            lpstrCommand: *const i8,
            lpstrReturnString: *mut i8,
            uReturnLength: u32,
            hwndCallback: usize,
        ) -> u32;
    }

    pub fn play_file(path: &str) {
        stop_file();
        let open_cmd = format!("open \"{}\" type mpegvideo alias ngs_bgm", path);
        if let Ok(c_open) = CString::new(open_cmd) {
            unsafe {
                mciSendStringA(c_open.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
        if let Ok(c_play) = CString::new("play ngs_bgm from 0 repeat") {
            unsafe {
                mciSendStringA(c_play.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
    }

    pub fn stop_file() {
        if let Ok(c_stop) = CString::new("stop ngs_bgm") {
            unsafe {
                mciSendStringA(c_stop.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
        if let Ok(c_close) = CString::new("close ngs_bgm") {
            unsafe {
                mciSendStringA(c_close.as_ptr(), std::ptr::null_mut(), 0, 0);
            }
        }
    }
}

pub struct JukeboxEngine {
    pub audio_out: Option<AudioOutput>,
    pub playing: bool,
    pub current_track_id: u8,
    pub current_event_key: Option<AudioEventKey>,
    pub last_playback_confirmed_audible: bool,
    pub mci_playing_file: Option<String>,
    pub mute: [bool; 5],
    pub solo: [bool; 5],
    pub oscilloscope: Vec<f32>,
    temp_samples: Vec<f32>,
}

impl Default for JukeboxEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for JukeboxEngine {
    fn drop(&mut self) {
        self.stop_external_file();
    }
}

impl JukeboxEngine {
    pub fn new() -> Self {
        let audio_out = Some(AudioOutput::new());
        Self {
            audio_out,
            playing: false,
            current_track_id: 1,
            current_event_key: None,
            last_playback_confirmed_audible: false,
            mci_playing_file: None,
            mute: [false; 5],
            solo: [false; 5],
            oscilloscope: vec![0.0; 256],
            temp_samples: Vec::with_capacity(4096),
        }
    }

    #[allow(dead_code)]
    pub fn new_headless() -> Self {
        Self {
            audio_out: None,
            playing: false,
            current_track_id: 1,
            current_event_key: None,
            last_playback_confirmed_audible: false,
            mci_playing_file: None,
            mute: [false; 5],
            solo: [false; 5],
            oscilloscope: vec![0.0; 256],
            temp_samples: Vec::with_capacity(4096),
        }
    }

    pub fn sample_rate(&self) -> u32 {
        self.audio_out.as_ref().map(|a| a.sample_rate()).unwrap_or(48_000)
    }

    pub fn play_external_file(&mut self, path: &str) {
        #[cfg(target_os = "windows")]
        {
            winmm::play_file(path);
        }
        self.mci_playing_file = Some(path.to_string());
    }

    pub fn stop_external_file(&mut self) {
        if self.mci_playing_file.is_some() {
            #[cfg(target_os = "windows")]
            {
                winmm::stop_file();
            }
            self.mci_playing_file = None;
        }
    }

    pub fn sync_channel_masks(&self, project: &mut NgsProject) {
        let any_solo = self.solo.iter().any(|&s| s);
        for ch in 0..5 {
            let enabled = if any_solo {
                self.solo[ch]
            } else {
                !self.mute[ch]
            };
            project.nes_live.bus.apu.channel_enabled[ch] = enabled;
        }
    }

    /// Dispara la reproducción de un evento de audio identificado por su `AudioEventKey` (trigger + comando + banco).
    pub fn trigger_audio_event(&mut self, project: &mut NgsProject, key: &AudioEventKey) {
        let sound_id = key.command_id;
        self.current_track_id = sound_id;
        self.current_event_key = Some(key.clone());

        // 1. Si el evento tiene reemplazo .MP3/.WAV, reproducirlo directamente
        if let Some(rep) = project.get_event_replacement(key).cloned() {
            self.play_external_file(&rep.external_file_path);
            self.last_playback_confirmed_audible = true;
            self.playing = true;
            return;
        }

        // 2. Si no, detener cualquier audio externo y usar la APU de la NES
        self.stop_external_file();

        // Restaurar el estado aislado del evento o el estado base del proyecto (evita contaminación entre sonidos consecutivos)
        if let Some(saved_state) = project.get_event_state(key).cloned() {
            let _ = project.nes_live.from_save_state_bytes(&saved_state);
        } else if let Some(base_state) = &project.base_audio_save_state {
            let _ = project.nes_live.from_save_state_bytes(base_state);
        } else if let Some(sc) = project.scenes.first() {
            if !sc.save_state.is_empty() {
                let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
            }
        }

        let event_kind = project
            .discovered_tracks
            .iter()
            .find(|t| &t.key == key)
            .map(|t| t.kind.clone())
            .unwrap_or(AudioEventKind::Unknown);

        // Si es un efecto de sonido (SFX) y el estado base tenía música sonando (ej. Super Mario Bros. 3 o Zelda),
        // silenciar la cola de música activa antes de disparar el SFX para no mezclar BGM con el efecto.
        if matches!(event_kind, AudioEventKind::Sfx) {
            silence_active_bgm_for_isolated_sfx(project, &key.trigger);
        }

        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.sample_rate());

        match key.trigger {
            AudioTrigger::Subroutine(sub_addr) => {
                call_subroutine_with_a(&mut project.nes_live, sub_addr, sound_id);
            }
            AudioTrigger::RamAddress(ram_addr) => {
                let target = (ram_addr as usize) & 0x7FF;
                if ram_addr == 0x00F0 {
                    // En Chip 'n Dale 2, limpiar el cerrojo $00B6 antes de escribir en $00F0
                    project.nes_live.bus.cpu_ram[0x00B6] = 0;
                    project.nes_live.bus.cpu_ram[0x00F0] = sound_id;
                } else {
                    project.nes_live.bus.cpu_ram[target] = sound_id;
                }
            }
            AudioTrigger::DriverCommand {
                entry_point,
                queue_address,
            } => {
                let target = (queue_address as usize) & 0x7FF;
                project.nes_live.bus.cpu_ram[target] = sound_id;
                call_subroutine_with_a(&mut project.nes_live, entry_point, sound_id);
            }
            AudioTrigger::Unknown => {
                if !project.sound_cmd_ram_addrs.is_empty() {
                    for &addr in &project.sound_cmd_ram_addrs {
                        let target = (addr as usize) & 0x7FF;
                        project.nes_live.bus.cpu_ram[target] = sound_id;
                    }
                }
            }
        }

        // Avanzar 4 frames para que el controlador APU absorba el comando y comprobar si hay salida audible
        let mut check_samples = Vec::with_capacity(512);
        for _ in 0..4 {
            project.nes_live.step_frame();
            project.nes_live.take_audio_samples(&mut check_samples);
        }
        let early_energy: f32 = check_samples.iter().map(|s| s.abs()).sum();
        self.last_playback_confirmed_audible = early_energy > 1.0
            || apu_frame_sig(&project.nes_live) != (0, 0, 0, 0);

        self.playing = true;
    }

    /// Adaptador compatible que busca el `AudioEventKey` correspondiente al `sound_id` (o construye uno con el trigger primario)
    #[allow(dead_code)]
    pub fn trigger_game_sound_id(&mut self, project: &mut NgsProject, sound_id: u8) {
        let key = project
            .discovered_tracks
            .iter()
            .find(|t| t.id == sound_id)
            .map(|t| t.key.clone())
            .unwrap_or_else(|| AudioEventKey::new(project.sound_trigger.clone(), sound_id));
        self.trigger_audio_event(project, &key);
    }

    /// Ejecuta 1 frame de la CPU 6502 + APU real del juego si la rockola está en reproducción
    pub fn step_real_audio_frame(&mut self, project: &mut NgsProject) {
        if !self.playing || !project.loaded {
            return;
        }

        if self.mci_playing_file.is_some() {
            // Animación suave del osciloscopio mientras suena un MP3/WAV externo
            let t = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as f32 * 0.015)
                .unwrap_or(0.0);
            for (i, val) in self.oscilloscope.iter_mut().enumerate() {
                *val = ((i as f32 * 0.12 + t).sin() * 0.28).clamp(-1.0, 1.0);
            }
            return;
        }

        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.sample_rate());
        self.sync_channel_masks(project);

        // Si la cola de audio tiene menos de ~3 frames, avanzar la emulación 6502 + APU
        let queued = self.audio_out.as_ref().map(|a| a.queued_samples()).unwrap_or(0);
        if queued < 3000 {
            project.nes_live.step_frame();
            self.temp_samples.clear();
            project.nes_live.take_audio_samples(&mut self.temp_samples);

            // Actualizar osciloscopio con el canal izquierdo
            for (i, chunk) in self.temp_samples.chunks_exact(2).take(self.oscilloscope.len()).enumerate() {
                self.oscilloscope[i] = chunk[0];
            }

            if let Some(ao) = &mut self.audio_out {
                ao.push_samples(&self.temp_samples);
            }
        }
    }

    /// Graba 5 segundos (300 frames) del audio real generado por la APU del juego a un archivo .WAV
    pub fn export_real_nes_audio_to_wav(&mut self, project: &mut NgsProject, path: &Path) -> Result<(), String> {
        if !project.loaded {
            return Err("Primero abre un juego desde el menú Archivo.".to_string());
        }
        let sample_rate = 44_100u32;
        project.nes_live.bus.apu.set_sample_rate(sample_rate);
        self.sync_channel_masks(project);

        let mut recorded_mono = Vec::with_capacity(sample_rate as usize * 5);
        let mut frame_buf = Vec::new();
        for _ in 0..300 {
            project.nes_live.step_frame();
            frame_buf.clear();
            project.nes_live.take_audio_samples(&mut frame_buf);
            for pair in frame_buf.chunks_exact(2) {
                recorded_mono.push((pair[0] + pair[1]) * 0.5);
            }
        }

        let data_bytes = (recorded_mono.len() * 2) as u32;
        let mut f = File::create(path).map_err(|e| e.to_string())?;
        f.write_all(b"RIFF").map_err(|e| e.to_string())?;
        f.write_all(&(36 + data_bytes).to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(b"WAVEfmt ").map_err(|e| e.to_string())?;
        f.write_all(&16u32.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&1u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&sample_rate.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&(sample_rate * 2).to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&2u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(&16u16.to_le_bytes()).map_err(|e| e.to_string())?;
        f.write_all(b"data").map_err(|e| e.to_string())?;
        f.write_all(&data_bytes.to_le_bytes()).map_err(|e| e.to_string())?;

        for s in recorded_mono {
            let pcm = (s.clamp(-0.98, 0.98) * 31000.0) as i16;
            f.write_all(&pcm.to_le_bytes()).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jukebox_engine_plays_distinct_tracks() {
        let test_games = [
            (r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Contra (USA).nes", vec![0x26u8, 0x2A, 0x2E, 0x3E, 0x10]),
            (r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Super Mario Bros. 3 (USA) (Rev 1).nes", vec![0x0Du8, 0x01, 0x02, 0x03, 0x04]),
            (r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Castlevania III - Dracula's Curse (USA).nes", vec![0x4Bu8, 0x63, 0x09, 0x0C, 0x35]),
            (r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Batman - Return of the Joker (USA).nes", vec![0x03u8, 0x05, 0x06, 0x09, 0x0B]),
            (r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Ninja Gaiden II - The Dark Sword of Chaos (USA).nes", vec![0x05u8, 0x06, 0x09, 0x0A, 0x0B]),
        ];

        let mut jukebox = JukeboxEngine::new_headless();

        for (rom_path, tracks_to_test) in &test_games {
            let path = std::path::Path::new(rom_path);
            if !path.exists() {
                continue;
            }
            let mut project = NgsProject::default();
            crate::parser::load_nes_rom_into_project(path, &mut project).unwrap();

            println!("\n--- Verificando Jukebox en: {} (Trigger: {:?}) ---", project.game_title, project.sound_trigger);
            assert_ne!(project.sound_trigger, SoundTrigger::Unknown, "Debe detectar un disparador para {}", project.game_title);

            let mut track_signatures = Vec::new();
            for &tid in tracks_to_test {
                jukebox.trigger_game_sound_id(&mut project, tid);
                let mut p1_periods = Vec::new();
                for _ in 0..40 {
                    project.nes_live.step_frame();
                    p1_periods.push((
                        project.nes_live.bus.apu.pulse1.timer_period,
                        project.nes_live.bus.apu.pulse2.timer_period,
                        project.nes_live.bus.apu.triangle.timer_period,
                    ));
                }
                println!("  Pista #{:02X} -> Muestra de períodos APU: {:?}", tid, &p1_periods[10..15]);
                if !track_signatures.contains(&p1_periods) {
                    track_signatures.push(p1_periods);
                }
            }
            assert!(
                track_signatures.len() >= 3,
                "{}: se esperaba escuchar melodías distintas en las pistas, pero solo se obtuvieron {} firmas únicas",
                project.game_title,
                track_signatures.len()
            );
            println!("  ✔ {} pasó con éxito: {} melodías distintas generadas por la APU!", project.game_title, track_signatures.len());
        }
    }

    #[test]
    fn test_classify_bgm_vs_sfx_by_duration() {
        let rom_path = r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Contra (USA).nes";
        let bytes = std::fs::read(rom_path).unwrap();
        let mut nes = Nes::default();
        nes.load_rom_bytes("Contra", &bytes).unwrap();
        nes.bus.apu.set_sample_rate(44100);

        for _ in 0..80 {
            nes.step_frame();
            nes.bus.apu.audio_samples.clear();
        }
        let base_state = nes.to_save_state_bytes();

        println!("\n=== Clasificación Real BGM vs SFX en Contra ===");
        let mut bgm_list = Vec::new();
        let mut sfx_list = Vec::new();

        for test_id in 1..=0x58u8 {
            nes.from_save_state_bytes(&base_state).unwrap();
            let _ = call_subroutine_with_a(&mut nes, 0xC16B, test_id);

            // Fase 1: primeros 30 frames
            let mut samples_early = Vec::new();
            for _ in 0..30 {
                nes.step_frame();
                nes.take_audio_samples(&mut samples_early);
            }
            let energy_early: f32 = samples_early.iter().map(|s| s.abs()).sum();

            // Fase 2: frames 31..70 (verificar si la música sigue sonando en bucle o si se apagó)
            let mut samples_late = Vec::new();
            for _ in 0..40 {
                nes.step_frame();
                nes.take_audio_samples(&mut samples_late);
            }
            let energy_late: f32 = samples_late.iter().map(|s| s.abs()).sum();

            if energy_early > 50.0 && energy_late > 300.0 {
                println!("  -> BGM (Melodía Continua): ID #0x{:02X} (Early={:.0}, Late={:.0})", test_id, energy_early, energy_late);
                bgm_list.push(test_id);
            } else if energy_early > 50.0 && energy_late <= 300.0 {
                println!("  -> SFX (Efecto Corto):     ID #0x{:02X} (Early={:.0}, Late={:.0})", test_id, energy_early, energy_late);
                sfx_list.push(test_id);
            }
        }

        println!("\nTotal Melodías Reales (BGM): {} -> {:02X?}", bgm_list.len(), bgm_list);
        println!("Total Efectos Reales (SFX):  {} -> {:02X?}", sfx_list.len(), sfx_list);
    }

    #[test]
    fn test_zelda_and_3stage_bgm_detector() {
        let ines = std::fs::read(r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Chip 'n Dale - Rescue Rangers 2 (USA).nes").unwrap();
        let mut nes = Nes::default();
        nes.load_rom_bytes("T", &ines).unwrap();
        nes.bus.apu.set_sample_rate(44100);
        for _ in 0..80 { nes.step_frame(); nes.bus.apu.audio_samples.clear(); }
        let base = nes.to_save_state_bytes();
        let mut bgm = Vec::new();
        let mut sfx = Vec::new();
        for id in 0x22..=0x78u8 {
            nes.from_save_state_bytes(&base).unwrap();
            nes.bus.cpu_ram[0x00F0] = id;
            let mut early = Vec::new();
            for _ in 0..25 { nes.step_frame(); nes.take_audio_samples(&mut early); }
            let e1: f32 = early.iter().map(|s| s.abs()).sum();
            if e1 < 40.0 { continue; }
            for _ in 25..110 { nes.step_frame(); nes.bus.apu.audio_samples.clear(); }
            let mut late = Vec::new();
            for _ in 0..35 { nes.step_frame(); nes.take_audio_samples(&mut late); }
            let e2: f32 = late.iter().map(|s| s.abs()).sum();
            if e2 > 800.0 { bgm.push(id); } else { sfx.push(id); }
        }
        println!("ChipDale ($00F0) -> BGM ({}): {:02X?} | SFX ({}): {:02X?}", bgm.len(), bgm, sfx.len(), &sfx[..sfx.len().min(15)]);
    }

    #[test]
    fn test_smb3_sfx_does_not_trigger_or_continue_bgm() {
        let rom_path = std::path::Path::new(
            r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Super Mario Bros. 3 (USA) (Rev 1).nes",
        );
        if !rom_path.exists() {
            return;
        }

        let mut project = NgsProject::default();
        crate::parser::load_nes_rom_into_project(rom_path, &mut project).unwrap();
        let mut jukebox = JukeboxEngine::new_headless();

        // 1. Verificar que Super Mario Bros. 3 registra triggers independientes para BGM ($04F5, $04F4) y SFX ($04F1, $04F2, $04F3, $04F6)
        assert!(
            project.audio_triggers.contains(&AudioTrigger::RamAddress(0x04F5)),
            "SMB3 debe registrar el trigger BGM RAM $04F5"
        );
        assert!(
            project.audio_triggers.contains(&AudioTrigger::RamAddress(0x04F4)),
            "SMB3 debe registrar el trigger BGM Mapas RAM $04F4"
        );
        assert!(
            project.audio_triggers.contains(&AudioTrigger::RamAddress(0x04F1)),
            "SMB3 debe registrar el trigger SFX RAM $04F1"
        );
        assert!(
            project.audio_triggers.contains(&AudioTrigger::RamAddress(0x04F2)),
            "SMB3 debe registrar el trigger SFX RAM $04F2"
        );

        // 2. Verificar que el mismo ID hexadecimal ($01) existe simultáneamente como BGM ($04F5::$01 y $04F4::$01) y como SFX ($04F1::$01 y $04F2::$01)
        let bgm_01_key = AudioEventKey::new(AudioTrigger::RamAddress(0x04F5), 0x01);
        let bgm_map1_key = AudioEventKey::new(AudioTrigger::RamAddress(0x04F4), 0x01);
        let sfx_jump_key = AudioEventKey::new(AudioTrigger::RamAddress(0x04F1), 0x01);
        let sfx_coin_key = AudioEventKey::new(AudioTrigger::RamAddress(0x04F2), 0x01);

        assert!(
            project.discovered_tracks.iter().any(|t| t.key == bgm_01_key && t.kind == AudioEventKind::Music),
            "Debe existir BGM $04F5::$01 (Overworld) como Music"
        );
        assert!(
            project.discovered_tracks.iter().any(|t| t.key == bgm_map1_key && t.kind == AudioEventKind::Music),
            "Debe existir BGM $04F4::$01 (Mapa Mundo 1) como Music"
        );
        assert!(
            project.discovered_tracks.iter().any(|t| t.key == sfx_jump_key && t.kind == AudioEventKind::Sfx),
            "Debe existir SFX $04F1::$01 (Salto) como Sfx"
        );
        assert!(
            project.discovered_tracks.iter().any(|t| t.key == sfx_coin_key && t.kind == AudioEventKind::Sfx),
            "Debe existir SFX $04F2::$01 (Moneda) como Sfx"
        );

        // 3. Reproducir primero una pista BGM ($04F5::$01) y comprobar que sigue sonando de forma sostenida en frames 90..120
        jukebox.trigger_audio_event(&mut project, &bgm_01_key);
        assert!(jukebox.last_playback_confirmed_audible, "BGM $04F5::$01 debe confirmar salida audible");
        for _ in 0..90 {
            project.nes_live.step_frame();
            project.nes_live.bus.apu.audio_samples.clear();
        }
        let mut bgm_late_samples = Vec::new();
        for _ in 90..120 {
            project.nes_live.step_frame();
            project.nes_live.take_audio_samples(&mut bgm_late_samples);
        }
        let bgm_late_energy: f32 = bgm_late_samples.iter().map(|s| s.abs()).sum();
        println!("SMB3 BGM ($04F5::$01) energía tardía (frames 90..120) = {:.1}", bgm_late_energy);
        assert!(bgm_late_energy > 400.0, "La pista BGM de SMB3 debe seguir sonando en los frames 90..120");

        // 4. Ahora disparar el SFX de Salto ($04F1::$01) y el de Moneda ($04F2::$01):
        //    - Debe producir energía audible al inicio (frames 0..25)
        //    - Y NO debe dejar ninguna canción BGM sonando después de que termina el efecto (frames 90..120)
        for sfx_key in [&sfx_jump_key, &sfx_coin_key] {
            jukebox.trigger_audio_event(&mut project, sfx_key);
            assert!(
                jukebox.last_playback_confirmed_audible,
                "El SFX {} debe confirmar salida audible inicial",
                sfx_key.short_label()
            );

            let mut sfx_early_samples = Vec::new();
            for _ in 0..25 {
                project.nes_live.step_frame();
                project.nes_live.take_audio_samples(&mut sfx_early_samples);
            }
            let sfx_early_energy: f32 = sfx_early_samples.iter().map(|s| s.abs()).sum();

            for _ in 25..90 {
                project.nes_live.step_frame();
                project.nes_live.bus.apu.audio_samples.clear();
            }
            let mut sfx_late_samples = Vec::new();
            for _ in 90..120 {
                project.nes_live.step_frame();
                project.nes_live.take_audio_samples(&mut sfx_late_samples);
            }
            let sfx_late_energy: f32 = sfx_late_samples.iter().map(|s| s.abs()).sum();

            println!(
                "SMB3 SFX ({}) -> energía temprana={:.1}, energía tardía (frames 90..120)={:.1}",
                sfx_key.short_label(),
                sfx_early_energy,
                sfx_late_energy
            );
            assert!(
                sfx_early_energy > 30.0,
                "El SFX {} debe sonar claramente en los primeros 25 frames",
                sfx_key.short_label()
            );
            assert!(
                sfx_late_energy < 10.0,
                "El SFX {} NO debe activar ni dejar sonando música BGM en los frames 90..120 (energía={:.1})",
                sfx_key.short_label(),
                sfx_late_energy
            );
        }
    }
}

