use crate::project::{DiscoveredTrack, NgsProject, SoundTrigger};
use nes_emu_rust::nes::Nes;
use nes_emu_rust::ui::audio::AudioOutput;
use std::fs::File;
use std::io::Write;
use std::path::Path;

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
        project.sound_trigger = SoundTrigger::Subroutine(0xC16B);
        project.discovered_tracks = vec![
            DiscoveredTrack { id: 0x26, title: "Pista 26 — Pantalla de Título & Menú".into(), is_music: true },
            DiscoveredTrack { id: 0x2A, title: "Pista 2A — Nivel 1: Selva / Jungla".into(), is_music: true },
            DiscoveredTrack { id: 0x3E, title: "Pista 3E — Nivel 2: Base Militar 3D".into(), is_music: true },
            DiscoveredTrack { id: 0x2E, title: "Pista 2E — Nivel 3: Cascada".into(), is_music: true },
            DiscoveredTrack { id: 0x32, title: "Pista 32 — Nivel 5: Campo de Nieve".into(), is_music: true },
            DiscoveredTrack { id: 0x36, title: "Pista 36 — Nivel 6: Zona Energética".into(), is_music: true },
            DiscoveredTrack { id: 0x3A, title: "Pista 3A — Nivel 7: Hangar".into(), is_music: true },
            DiscoveredTrack { id: 0x42, title: "Pista 42 — Nivel 8: Guarida Alienígena".into(), is_music: true },
            DiscoveredTrack { id: 0x46, title: "Pista 46 — Batalla de Jefe (Boss Fight)".into(), is_music: true },
            DiscoveredTrack { id: 0x4E, title: "Pista 4E — Nivel Completado (Fanfarria)".into(), is_music: true },
            DiscoveredTrack { id: 0x52, title: "Pista 52 — Final y Créditos".into(), is_music: true },
            DiscoveredTrack { id: 0x54, title: "Pista 54 — Game Over".into(), is_music: true },
            DiscoveredTrack { id: 0x10, title: "SFX 10 — Disparo de Rifle Normal".into(), is_music: false },
            DiscoveredTrack { id: 0x11, title: "SFX 11 — Disparo Spread Gun (Escopeta)".into(), is_music: false },
            DiscoveredTrack { id: 0x12, title: "SFX 12 — Disparo de Rayo Láser".into(), is_music: false },
            DiscoveredTrack { id: 0x13, title: "SFX 13 — Disparo de Fuego / Lanzallamas".into(), is_music: false },
            DiscoveredTrack { id: 0x15, title: "SFX 15 — Explosión Pequeña".into(), is_music: false },
            DiscoveredTrack { id: 0x16, title: "SFX 16 — Explosión Grande / Jefe".into(), is_music: false },
            DiscoveredTrack { id: 0x19, title: "SFX 19 — Halcón de Arma en Vuelo".into(), is_music: false },
            DiscoveredTrack { id: 0x1A, title: "SFX 1A — Salto del Soldado".into(), is_music: false },
            DiscoveredTrack { id: 0x1E, title: "SFX 1E — Daño / Caída de Soldado".into(), is_music: false },
            DiscoveredTrack { id: 0x24, title: "SFX 24 — Alarma / Sirena de la Base".into(), is_music: false },
            DiscoveredTrack { id: 0x25, title: "SFX 25 — Rugido de Jefe Alien".into(), is_music: false },
            DiscoveredTrack { id: 0x55, title: "SFX 55 — Vida Extra (1-UP)".into(), is_music: false },
        ];
        return;
    }

    if title_lower.contains("mario") {
        project.sound_trigger = SoundTrigger::RamAddress(0x04F5);
        project.discovered_tracks = vec![
            DiscoveredTrack { id: 0x0D, title: "Pista 0D — Pantalla de Título".into(), is_music: true },
            DiscoveredTrack { id: 0xB0, title: "Pista B0 — Mapa del Mundo 1 (Pradera)".into(), is_music: true },
            DiscoveredTrack { id: 0x01, title: "Pista 01 — Nivel Superficie (Overworld)".into(), is_music: true },
            DiscoveredTrack { id: 0x02, title: "Pista 02 — Nivel Subterráneo (Underground)".into(), is_music: true },
            DiscoveredTrack { id: 0x03, title: "Pista 03 — Nivel Acuático (Underwater)".into(), is_music: true },
            DiscoveredTrack { id: 0x04, title: "Pista 04 — Fortaleza / Castillo".into(), is_music: true },
            DiscoveredTrack { id: 0x05, title: "Pista 05 — Barco Volador (Airship)".into(), is_music: true },
            DiscoveredTrack { id: 0x06, title: "Pista 06 — Estrella de Invencibilidad (Starman)".into(), is_music: true },
            DiscoveredTrack { id: 0x07, title: "Pista 07 — Batalla de Jefe Koopaling".into(), is_music: true },
            DiscoveredTrack { id: 0x08, title: "Pista 08 — Fanfarria de Nivel Completado".into(), is_music: true },
            DiscoveredTrack { id: 0x0E, title: "Pista 0E — Casa de Toad".into(), is_music: true },
            DiscoveredTrack { id: 0x0F, title: "Pista 0F — Batalla contra Bowser".into(), is_music: true },
            DiscoveredTrack { id: 0x0C, title: "Pista 0C — Game Over".into(), is_music: true },
            DiscoveredTrack { id: 0x20, title: "SFX 20 — Salto Pequeño".into(), is_music: false },
            DiscoveredTrack { id: 0x21, title: "SFX 21 — Salto Super Mario".into(), is_music: false },
            DiscoveredTrack { id: 0x22, title: "SFX 22 — Moneda".into(), is_music: false },
            DiscoveredTrack { id: 0x23, title: "SFX 23 — Champiñón / Power Up".into(), is_music: false },
            DiscoveredTrack { id: 0x24, title: "SFX 24 — Bola de Fuego".into(), is_music: false },
            DiscoveredTrack { id: 0x25, title: "SFX 25 — Pisotón a Enemigo".into(), is_music: false },
            DiscoveredTrack { id: 0x26, title: "SFX 26 — Caída por Tubería".into(), is_music: false },
        ];
        return;
    }

    if title_lower.contains("castlevania") || title_lower.contains("akumajou") {
        let sub = if title_lower.contains("akumajou") { 0xE249 } else { 0xE25F };
        project.sound_trigger = SoundTrigger::Subroutine(sub);
        project.discovered_tracks = vec![
            DiscoveredTrack { id: 0x4B, title: "Pista 4B — Prelude (Pantalla de Título)".into(), is_music: true },
            DiscoveredTrack { id: 0x63, title: "Pista 63 — Beginning (Nivel 1)".into(), is_music: true },
            DiscoveredTrack { id: 0x09, title: "Pista 09 — Clockwork (Torre del Reloj)".into(), is_music: true },
            DiscoveredTrack { id: 0x0C, title: "Pista 0C — Mad Forest (Bosque Maldito)".into(), is_music: true },
            DiscoveredTrack { id: 0x46, title: "Pista 46 — Stream (Río Subterráneo)".into(), is_music: true },
            DiscoveredTrack { id: 0x3A, title: "Pista 3A — Aquarius (Templo Sumergido)".into(), is_music: true },
            DiscoveredTrack { id: 0x6C, title: "Pista 6C — Anxiety (Cueva)".into(), is_music: true },
            DiscoveredTrack { id: 0x3D, title: "Pista 3D — Rising (Castillo Exterior)".into(), is_music: true },
            DiscoveredTrack { id: 0x3B, title: "Pista 3B — Dead Beat (Pantano)".into(), is_music: true },
            DiscoveredTrack { id: 0x35, title: "Pista 35 — Boss Fight (Batalla de Jefe)".into(), is_music: true },
            DiscoveredTrack { id: 0x2C, title: "Pista 2C — Fanfarria de Victoria".into(), is_music: true },
            DiscoveredTrack { id: 0x6B, title: "Pista 6B — Game Over".into(), is_music: true },
            DiscoveredTrack { id: 0x01, title: "SFX 01 — Latigazo".into(), is_music: false },
            DiscoveredTrack { id: 0x02, title: "SFX 02 — Salto".into(), is_music: false },
            DiscoveredTrack { id: 0x03, title: "SFX 03 — Recoger Corazón".into(), is_music: false },
            DiscoveredTrack { id: 0x04, title: "SFX 04 — Golpe / Daño".into(), is_music: false },
        ];
        return;
    }

    if title_lower.contains("batman") {
        project.sound_trigger = SoundTrigger::RamAddress(0x0042);
        let bgm_ids = [0x01u8, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
                is_music: true,
            });
        }
        let sfx_ids = [0x20u8, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
                is_music: false,
            });
        }
        return;
    }

    if title_lower.contains("ninja") {
        project.sound_trigger = SoundTrigger::RamAddress(0x0700);
        let bgm_ids = [0x04u8, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x1E, 0x2A, 0x33];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
                is_music: true,
            });
        }
        let sfx_ids = [0x0Eu8, 0x0F, 0x10, 0x19, 0x1A, 0x1F, 0x20, 0x21, 0x22, 0x23, 0x24, 0x25];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
                is_music: false,
            });
        }
        return;
    }

    if title_lower.contains("chip") || title_lower.contains("dale") {
        project.sound_trigger = SoundTrigger::RamAddress(0x00F0);
        let bgm_ids = [0x72u8, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
                is_music: true,
            });
        }
        let sfx_ids = [0x22u8, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2A, 0x2B, 0x2C, 0x2D];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
                is_music: false,
            });
        }
        return;
    }

    if title_lower.contains("tiny toon") {
        project.sound_trigger = SoundTrigger::Subroutine(0xF823);
        let bgm_ids = [0x28u8, 0x5E, 0x62, 0x6A, 0x6C];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
                is_music: true,
            });
        }
        let sfx_ids = [0x02u8, 0x03, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0A, 0x0B, 0x0C];
        for (idx, &id) in sfx_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", idx + 1, id),
                is_music: false,
            });
        }
        return;
    }

    if title_lower.contains("zelda") {
        project.sound_trigger = SoundTrigger::RamAddress(0x0600);
        project.discovered_tracks = vec![
            DiscoveredTrack { id: 0x01, title: "Melodía 01 — Tema de Superficie / Hyrule (ID $01)".into(), is_music: true },
            DiscoveredTrack { id: 0x02, title: "Melodía 02 — Mazmorra / Inframundo (ID $02)".into(), is_music: true },
            DiscoveredTrack { id: 0x04, title: "Melodía 03 — Mazmorra Final / Death Mountain (ID $04)".into(), is_music: true },
            DiscoveredTrack { id: 0x10, title: "Melodía 04 — Introducción / Pantalla de Título (ID $10)".into(), is_music: true },
            DiscoveredTrack { id: 0x20, title: "Melodía 05 — Game Over / Continuar (ID $20)".into(), is_music: true },
            DiscoveredTrack { id: 0x40, title: "Melodía 06 — Fanfarria de Trifuerza (ID $40)".into(), is_music: true },
            DiscoveredTrack { id: 0x80, title: "Melodía 07 — Rescate de Zelda y Créditos (ID $80)".into(), is_music: true },
            DiscoveredTrack { id: 0x11, title: "SFX 01 — Espada / Ataque ($0604=$01)".into(), is_music: false },
            DiscoveredTrack { id: 0x12, title: "SFX 02 — Bumerán / Flecha ($0604=$02)".into(), is_music: false },
            DiscoveredTrack { id: 0x14, title: "SFX 03 — Flauta / Magia ($0604=$04)".into(), is_music: false },
            DiscoveredTrack { id: 0x21, title: "SFX 04 — Secreto Descubierto ($0602=$01)".into(), is_music: false },
            DiscoveredTrack { id: 0x22, title: "SFX 05 — Explosión de Bomba ($0602=$02)".into(), is_music: false },
            DiscoveredTrack { id: 0x24, title: "SFX 06 — Golpe / Daño a Enemigo ($0602=$04)".into(), is_music: false },
            DiscoveredTrack { id: 0x28, title: "SFX 07 — Recoger Corazón / Rupia ($0602=$08)".into(), is_music: false },
        ];
        return;
    }

    if title_lower.contains("simpson") {
        project.sound_trigger = SoundTrigger::Subroutine(0xFAC7);
        for id in 0x01..=0x0Du8 {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("Melodía {:02} — Tema Musical (ID ${:02X})", id, id),
                is_music: true,
            });
        }
        for sfx_num in 0x01..=0x0Cu8 {
            let id = 0x80 | sfx_num;
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", sfx_num, sfx_num),
                is_music: false,
            });
        }
        return;
    }

    if title_lower.contains("kirby") {
        project.sound_trigger = SoundTrigger::Subroutine(0xF862);
        let bgm_ids = [0x01u8, 0x02, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F, 0x10, 0x11, 0x12, 0x14, 0x1C, 0x20, 0x21, 0x23];
        for (idx, &id) in bgm_ids.iter().enumerate() {
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("Melodía {:02} — Tema Musical (ID ${:02X})", idx + 1, id),
                is_music: true,
            });
        }
        for sfx_num in 0x01..=0x0Cu8 {
            let id = 0x80 | sfx_num;
            project.discovered_tracks.push(DiscoveredTrack {
                id,
                title: format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", sfx_num, sfx_num),
                is_music: false,
            });
        }
        return;
    }

    if title_lower.contains("battletoads") {
        project.sound_trigger = SoundTrigger::Subroutine(0x9148);
        project.discovered_tracks = vec![
            DiscoveredTrack { id: 0x01, title: "Pista 01 — Pantalla de Título (Battletoads Theme)".into(), is_music: true },
            DiscoveredTrack { id: 0x02, title: "Pista 02 — Ragnarok's Canyon (Nivel 1)".into(), is_music: true },
            DiscoveredTrack { id: 0x03, title: "Pista 03 — Wookie Hole (Nivel 2)".into(), is_music: true },
            DiscoveredTrack { id: 0x04, title: "Pista 04 — Turbo Tunnel (Nivel 3)".into(), is_music: true },
            DiscoveredTrack { id: 0x05, title: "Pista 05 — Arctic Caverns (Nivel 4)".into(), is_music: true },
            DiscoveredTrack { id: 0x06, title: "Pista 06 — Surf City (Nivel 5)".into(), is_music: true },
            DiscoveredTrack { id: 0x07, title: "Pista 07 — Karnath's Lair (Nivel 6)".into(), is_music: true },
            DiscoveredTrack { id: 0x08, title: "Pista 08 — Volkmire's Inferno (Nivel 7)".into(), is_music: true },
            DiscoveredTrack { id: 0x09, title: "Pista 09 — Intruder Excluder (Nivel 8)".into(), is_music: true },
            DiscoveredTrack { id: 0x0A, title: "Pista 0A — Batalla contra Dark Queen".into(), is_music: true },
            DiscoveredTrack { id: 0x10, title: "SFX 10 — Puñetazo Normal".into(), is_music: false },
            DiscoveredTrack { id: 0x11, title: "SFX 11 — Super Golpe / Bota Gigante".into(), is_music: false },
            DiscoveredTrack { id: 0x12, title: "SFX 12 — Salto".into(), is_music: false },
            DiscoveredTrack { id: 0x13, title: "SFX 13 — Impacto / Explosión".into(), is_music: false },
            DiscoveredTrack { id: 0x14, title: "SFX 14 — Pausa (Beatbox)".into(), is_music: false },
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

    // Si tenemos estado base, probar los candidatos más probables
    if let Some(base_state) = &project.base_audio_save_state {
        // Probar candidatos de subrutina (JSR $XXXX)
        for (sub, imms) in top_jsr.iter().take(5) {
            let mut distinct_sigs = Vec::new();
            for &test_id in imms.iter().take(5) {
                let _ = project.nes_live.from_save_state_bytes(base_state);
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
                project.sound_trigger = SoundTrigger::Subroutine(*sub);
                classify_tracks_via_apu_energy(project, imms);
                return;
            }
        }

        // Probar candidatos de escritura en RAM (STA $XXXX)
        for (addr, imms) in top_sta.iter().take(5) {
            let mut distinct_sigs = Vec::new();
            for &test_id in imms.iter().take(5) {
                let _ = project.nes_live.from_save_state_bytes(base_state);
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
                project.sound_trigger = SoundTrigger::RamAddress(*addr);
                classify_tracks_via_apu_energy(project, imms);
                return;
            }
        }
    }

    if let Some((sub, imms)) = top_jsr.first() {
        project.sound_trigger = SoundTrigger::Subroutine(*sub);
        classify_tracks_via_apu_energy(project, imms);
    } else if let Some((addr, imms)) = top_sta.first() {
        project.sound_trigger = SoundTrigger::RamAddress(*addr);
        classify_tracks_via_apu_energy(project, imms);
    } else {
        project.sound_trigger = SoundTrigger::Unknown;
        fallback_discovered_tracks(project);
    }
}

/// Clasifica dinámicamente las pistas comparando contra la línea base del estado guardado:
/// 1. Graba la firma APU de referencia (`base_early`, `base_late`) sin disparar ningún ID.
/// 2. Para cada candidato, mide si realmente alteró la APU respecto a la línea base (descarta IDs inválidos/no-op).
/// 3. Distingue Melodía Continua (BGM) de Efecto de Sonido (SFX):
///    - BGM: sigue activa en la fase tardía (`energy_late > 300.0`), cambia al menos 2 canales melódicos (Pulse1, Pulse2, Triangle)
///      respecto a la línea base y mantiene notas tonales activas en múltiples canales.
///    - SFX: produce un cambio transitorio en la fase temprana que termina en la fase tardía, o solo altera 1 canal (como Noise o un barrido corto).
fn classify_tracks_via_apu_energy(project: &mut NgsProject, extra_imms: &[u8]) {
    let Some(base_state) = project.base_audio_save_state.clone() else {
        fallback_discovered_tracks(project);
        return;
    };

    let mut candidates: Vec<u8> = (1..=0x58u8).collect();
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
    let mut dummy_samples = Vec::with_capacity(2048);
    for _ in 0..18 {
        project.nes_live.step_frame();
        project.nes_live.take_audio_samples(&mut dummy_samples);
        base_early_sigs.push(apu_frame_sig(&project.nes_live));
    }
    for _ in 0..27 {
        project.nes_live.step_frame();
        project.nes_live.take_audio_samples(&mut dummy_samples);
        base_late_sigs.push(apu_frame_sig(&project.nes_live));
    }

    let mut seen_signatures: Vec<(Vec<(u16, u16, u16, u16)>, Vec<(u16, u16, u16, u16)>)> = Vec::new();

    for id in candidates {
        let _ = project.nes_live.from_save_state_bytes(&base_state);
        match project.sound_trigger {
            SoundTrigger::Subroutine(sub) => {
                if !call_subroutine_with_a(&mut project.nes_live, sub, id) {
                    continue;
                }
            }
            SoundTrigger::RamAddress(addr) => {
                project.nes_live.bus.cpu_ram[(addr as usize) & 0x7FF] = id;
            }
            SoundTrigger::Unknown => continue,
        }

        samples_early.clear();
        let mut early_sigs = Vec::with_capacity(18);
        for _ in 0..18 {
            project.nes_live.step_frame();
            project.nes_live.take_audio_samples(&mut samples_early);
            early_sigs.push(apu_frame_sig(&project.nes_live));
        }
        let energy_early: f32 = samples_early.iter().map(|s| s.abs()).sum();

        samples_late.clear();
        let mut late_sigs = Vec::with_capacity(27);
        for _ in 0..27 {
            project.nes_live.step_frame();
            project.nes_live.take_audio_samples(&mut samples_late);
            late_sigs.push(apu_frame_sig(&project.nes_live));
        }
        let energy_late: f32 = samples_late.iter().map(|s| s.abs()).sum();

        // Si no produjo sonido ni cambió nada respecto a la línea base, es un ID vacío/inválido
        if energy_early <= 35.0 || (early_sigs == base_early_sigs && late_sigs == base_late_sigs) {
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

        // Si parece candidato a canción a los 45 frames, verificar hasta los 2.1 segundos (frame 125)
        // que siga sonando con cambios de notas reales (para no confundir efectos largos de 1s con melodías).
        let mut is_music = false;
        if energy_late > 320.0 && melodic_channels_changed >= 1 && late_sigs != base_late_sigs {
            for _ in 45..95 {
                project.nes_live.step_frame();
                project.nes_live.bus.apu.audio_samples.clear();
            }
            let mut sustained_samples = Vec::with_capacity(2048);
            let mut distinct_notes = std::collections::HashSet::new();
            for _ in 95..125 {
                project.nes_live.step_frame();
                project.nes_live.take_audio_samples(&mut sustained_samples);
                let sig = apu_frame_sig(&project.nes_live);
                if sig.0 > 8 { distinct_notes.insert((1u8, sig.0)); }
                if sig.1 > 8 { distinct_notes.insert((2u8, sig.1)); }
                if sig.2 > 8 { distinct_notes.insert((3u8, sig.2)); }
            }
            let sustained_energy: f32 = sustained_samples.iter().map(|s| s.abs()).sum();
            if sustained_energy > 600.0 && distinct_notes.len() >= 3 {
                is_music = true;
            }
        }

        let count_so_far = project.discovered_tracks.iter().filter(|t| t.is_music == is_music).count() + 1;
        project.discovered_tracks.push(DiscoveredTrack {
            id,
            title: if is_music {
                format!("Melodía {:02} — Tema Musical (ID ${:02X})", count_so_far, id)
            } else {
                format!("SFX {:02} — Efecto de Sonido (ID ${:02X})", count_so_far, id)
            },
            is_music,
        });
    }

    let _ = project.nes_live.from_save_state_bytes(&base_state);
    if project.discovered_tracks.is_empty() {
        fallback_discovered_tracks(project);
    }
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

fn fallback_discovered_tracks(project: &mut NgsProject) {
    for id in 1..=16 {
        project.discovered_tracks.push(DiscoveredTrack {
            id,
            title: format!("Pista {:02X} — Tema / Música #{:02X}", id, id),
            is_music: true,
        });
    }
    for id in 0x10..=0x25 {
        project.discovered_tracks.push(DiscoveredTrack {
            id,
            title: format!("SFX {:02X} — Efecto de Sonido #{:02X}", id, id),
            is_music: false,
        });
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

    /// Dispara la reproducción de un tema o efecto. Si está vinculado a un archivo .MP3/.WAV lo reproduce con MCI,
    /// o de lo contrario ejecuta el motor sonoro APU del juego cargado.
    pub fn trigger_game_sound_id(&mut self, project: &mut NgsProject, sound_id: u8) {
        self.current_track_id = sound_id;

        // 1. Si la pista tiene reemplazo .MP3/.WAV, reproducirla directamente
        if let Some(rep) = project.audio_replacements.get(&sound_id) {
            self.play_external_file(&rep.external_file_path);
            self.playing = true;
            return;
        }

        // 2. Si no, detener cualquier audio externo y usar la APU de la NES
        self.stop_external_file();

        // Si tenemos un estado guardado específicamente para esta pista, restaurarlo
        if let Some(saved_state) = project.audio_track_states.get(&sound_id) {
            let _ = project.nes_live.from_save_state_bytes(saved_state);
        } else if let Some(base_state) = &project.base_audio_save_state {
            let _ = project.nes_live.from_save_state_bytes(base_state);
        } else if let Some(sc) = project.scenes.first() {
            if !sc.save_state.is_empty() {
                let _ = project.nes_live.from_save_state_bytes(&sc.save_state);
            }
        }

        project
            .nes_live
            .bus
            .apu
            .set_sample_rate(self.sample_rate());

        match project.sound_trigger {
            SoundTrigger::Subroutine(sub_addr) => {
                if sub_addr == 0xFAC7 && sound_id >= 0x80 {
                    // En The Simpsons, $FAC7 dispara melodías BGM y $FADA dispara efectos SFX
                    call_subroutine_with_a(&mut project.nes_live, 0xFADA, sound_id & 0x7F);
                } else if sub_addr == 0xF862 && sound_id >= 0x80 {
                    // En Kirby's Adventure, $F862 dispara melodías BGM y $F826 dispara efectos SFX
                    call_subroutine_with_a(&mut project.nes_live, 0xF826, sound_id & 0x7F);
                } else {
                    call_subroutine_with_a(&mut project.nes_live, sub_addr, sound_id);
                }
            }
            SoundTrigger::RamAddress(ram_addr) => {
                let target = (ram_addr as usize) & 0x7FF;
                if ram_addr == 0x04F5 && (0x20..=0x2F).contains(&sound_id) {
                    let bit = 1u8 << (sound_id & 0x07);
                    project.nes_live.bus.cpu_ram[0x04F2] = bit;
                    project.nes_live.bus.cpu_ram[0x04F4] = bit;
                } else if ram_addr == 0x0600 && [0x11, 0x12, 0x14].contains(&sound_id) {
                    // En The Legend of Zelda, $0600 es BGM y $0604/$0602 son colas de SFX
                    project.nes_live.bus.cpu_ram[0x0604] = sound_id & 0x0F;
                } else if ram_addr == 0x0600 && [0x21, 0x22, 0x24, 0x28].contains(&sound_id) {
                    project.nes_live.bus.cpu_ram[0x0602] = sound_id & 0x0F;
                } else if ram_addr == 0x00F0 {
                    // En Chip 'n Dale 2, limpiar el cerrojo $00B6 antes de escribir en $00F0
                    project.nes_live.bus.cpu_ram[0x00B6] = 0;
                    project.nes_live.bus.cpu_ram[0x00F0] = sound_id;
                } else {
                    project.nes_live.bus.cpu_ram[target] = sound_id;
                }
            }
            SoundTrigger::Unknown => {
                if !project.sound_cmd_ram_addrs.is_empty() {
                    for &addr in &project.sound_cmd_ram_addrs {
                        let target = (addr as usize) & 0x7FF;
                        project.nes_live.bus.cpu_ram[target] = sound_id;
                    }
                }
            }
        }

        // Avanzar 4 frames para que el controlador APU absorba el comando
        for _ in 0..4 {
            project.nes_live.step_frame();
            project.nes_live.bus.apu.audio_samples.clear();
        }

        self.playing = true;
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
            (r"c:\Users\Cesar\Documents\AI Proyectos\NesEmuRust\rooms\Super Mario Bros. 3 (USA) (Rev 1).nes", vec![0x0Du8, 0xB0, 0x01, 0x02, 0x03]),
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
}

