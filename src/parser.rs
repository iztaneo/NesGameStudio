use crate::project::{NgsProject, SubroutineInfo};
use nes_emu_rust::controller::JoypadButtons;
use std::fs;
use std::path::Path;

pub fn load_c_and_asm_into_project(c_path: &Path, project: &mut NgsProject) -> Result<String, String> {
    let raw_c = fs::read_to_string(c_path)
        .map_err(|e| format!("Error leyendo archivo C '{}': {}", c_path.display(), e))?;

    let stem = c_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Juego_NES")
        .to_string();

    *project = NgsProject::default();
    project.loaded = true;
    project.game_title = stem;
    project.c_file_path = Some(c_path.to_path_buf());
    project.raw_c_source = raw_c.clone();

    project.mapper_id = parse_define_u16(&raw_c, "#define MAPPER_ID").unwrap_or(0) as u8;
    // NesEmuRust exporta: `static uint8_t NAMETABLE_MIRRORING = 1; /* 0=Horizontal, 1=Vertical */`
    project.mirroring_mode = parse_assignment_u8(&raw_c, "NAMETABLE_MIRRORING")
        .or_else(|| parse_define_u16(&raw_c, "#define MIRRORING_MODE").map(|v| v as u8))
        .unwrap_or(0)
        & 1;
    project.vec_nmi = parse_define_u16(&raw_c, "#define VEC_NMI").unwrap_or(0x8000);
    project.vec_reset = parse_define_u16(&raw_c, "#define VEC_RESET").unwrap_or(0x8000);
    project.vec_irq = parse_define_u16(&raw_c, "#define VEC_IRQ").unwrap_or(0x8000);

    project.prg_rom = extract_c_byte_array(&raw_c, "PRG_ROM[");
    project.chr_mem = extract_c_byte_array(&raw_c, "CHR_MEM[");

    if project.prg_rom.is_empty() {
        return Err("El archivo .C seleccionado no contiene un arreglo PRG_ROM[] válido.".to_string());
    }
    if let Some(expected) = parse_define_usize(&raw_c, "#define PRG_ROM_SIZE").filter(|v| *v > 0) {
        if project.prg_rom.len() != expected {
            return Err(format!(
                "PRG_ROM[] leído con {} bytes pero el archivo declara {} bytes.",
                project.prg_rom.len(),
                expected
            ));
        }
    }

    let non_zero_chr = project.chr_mem.iter().filter(|&&b| b != 0).count();
    project.is_chr_ram = non_zero_chr <= 64;

    // Reconstruir imagen iNES en memoria desde PRG_ROM[] y CHR_MEM[] del archivo .C
    let ines_bytes = build_ines_from_extracted(
        &project.prg_rom,
        &project.chr_mem,
        project.mapper_id,
        project.mirroring_mode,
        project.is_chr_ram,
    );

    // Ejecutar la CPU 6502 + PPU + APU real para descomprimir los mapas de bits reales y capturar la música real
    populate_real_scenes_and_audio_from_ines(&ines_bytes, project)?;
    project.rebuild_global_tile_catalog();
    project.rebuild_global_tile_catalog();

    // Cargar subrutinas desde el .asm compañero o desde el propio .c
    let candidate_asm = c_path.with_extension("asm");
    project.subroutines.clear();
    if candidate_asm.exists() {
        project.asm_file_path = Some(candidate_asm.clone());
        let _ = load_asm_subroutines(&candidate_asm, project);
    } else {
        extract_subroutines_from_c(&raw_c, project);
    }

    Ok(format!(
        "Cargado '{}': PRG={} KB, {} ({} escenas reales descomprimidas), {} subrutinas (Mapper {})",
        project.game_title,
        project.prg_rom.len() / 1024,
        if project.is_chr_ram { "CHR-RAM Descomprimida por 6502" } else { "CHR-ROM" },
        project.scenes.len(),
        project.subroutines.len(),
        project.mapper_id
    ))
}

pub fn load_nes_rom_into_project(nes_path: &Path, project: &mut NgsProject) -> Result<String, String> {
    let ines_bytes = fs::read(nes_path)
        .map_err(|e| format!("Error leyendo ROM '{}': {}", nes_path.display(), e))?;
    if ines_bytes.len() < 16 || &ines_bytes[0..4] != b"NES\x1A" {
        return Err("El archivo seleccionado no tiene cabecera iNES válida.".to_string());
    }
    let stem = nes_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Juego_NES")
        .to_string();

    let prg_banks = ines_bytes[4] as usize;
    let chr_banks = ines_bytes[5] as usize;
    let flags6 = ines_bytes[6];
    let flags7 = ines_bytes[7];
    let mapper_id = (flags7 & 0xF0) | (flags6 >> 4);
    let prg_size = prg_banks * 16384;
    let chr_size = chr_banks * 8192;

    *project = NgsProject::default();
    project.loaded = true;
    project.game_title = stem;
    project.mapper_id = mapper_id;
    project.mirroring_mode = flags6 & 1;
    project.is_chr_ram = chr_banks == 0;
    if 16 + prg_size <= ines_bytes.len() {
        project.prg_rom = ines_bytes[16..16 + prg_size].to_vec();
    }
    if chr_size > 0 && 16 + prg_size + chr_size <= ines_bytes.len() {
        project.chr_mem = ines_bytes[16 + prg_size..16 + prg_size + chr_size].to_vec();
    } else {
        project.chr_mem = vec![0u8; 8192];
    }

    populate_real_scenes_and_audio_from_ines(&ines_bytes, project)?;
    project.rebuild_global_tile_catalog();

    // Generar también el código C y ASM base en memoria usando el desensamblador de nes_emu_rust
    let generated_c =
        nes_emu_rust::cpu::disassembler::export_full_c_source(&project.nes_live.bus, &project.game_title);
    extract_subroutines_from_c(&generated_c, project);
    project.raw_c_source = generated_c;

    Ok(format!(
        "ROM '{}' cargada: PRG={} KB, {} escenas con mapas de bits y música reales capturadas",
        project.game_title,
        project.prg_rom.len() / 1024,
        project.scenes.len()
    ))
}

fn build_ines_from_extracted(
    prg: &[u8],
    chr: &[u8],
    mapper_id: u8,
    mirroring_mode: u8,
    is_chr_ram: bool,
) -> Vec<u8> {
    let prg_banks = (prg.len() / 16384).max(1) as u8;
    let chr_banks = if is_chr_ram { 0u8 } else { (chr.len() / 8192).max(1) as u8 };
    let flags6 = ((mapper_id & 0x0F) << 4) | (mirroring_mode & 0x01);
    let flags7 = mapper_id & 0xF0;

    let mut ines = Vec::with_capacity(16 + prg.len() + chr.len());
    ines.extend_from_slice(b"NES\x1A");
    ines.push(prg_banks);
    ines.push(chr_banks);
    ines.push(flags6);
    ines.push(flags7);
    ines.extend_from_slice(&[0u8; 8]);
    ines.extend_from_slice(prg);
    if !is_chr_ram {
        ines.extend_from_slice(chr);
    }
    ines
}

/// Avanza la CPU 6502 + PPU + APU real sobre la ROM para que el propio juego inicialice sus mapas de bits
/// y active su motor musical real en la APU.
fn populate_real_scenes_and_audio_from_ines(
    ines_bytes: &[u8],
    project: &mut NgsProject,
) -> Result<(), String> {
    project.scenes.clear();
    project
        .nes_live
        .load_rom_bytes(project.game_title.clone(), ines_bytes)?;

    // 1. Avanzar 75 frames de arranque (inicialización de PPU y pantalla de inicio)
    for _ in 0..75 {
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }

    // Presionar START brevemente para avanzar la pantalla de inicio o demo del juego
    project.nes_live.set_controller1(JoypadButtons::START);
    for _ in 0..6 {
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }
    project.nes_live.set_controller1(JoypadButtons::empty());
    for _ in 0..35 {
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }

    // Capturar Estado 1: Pantalla de Inicio / Título
    project.capture_current_live_scene(
        "Pantalla de Inicio / Título (VRAM Activa)".to_string(),
    );
    project.base_audio_save_state = Some(project.nes_live.to_save_state_bytes());

    // 2. Presionar START otra vez para iniciar la partida
    let ram_before_start = project.nes_live.bus.cpu_ram;
    project.nes_live.set_controller1(JoypadButtons::START);
    for _ in 0..6 {
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }
    project.nes_live.set_controller1(JoypadButtons::empty());
    for _ in 0..10 {
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }
    let ram_after_start = project.nes_live.bus.cpu_ram;

    // Detectar qué direcciones de RAM cambiaron al disparar el comando de sonido de Start
    project.sound_cmd_ram_addrs.clear();
    for addr in 0x0080..0x01FFusize {
        if ram_before_start[addr] != ram_after_start[addr] && ram_after_start[addr] > 0 && ram_after_start[addr] < 0x40 {
            project.sound_cmd_ram_addrs.push(addr as u16);
        }
    }

    project.capture_current_live_scene(
        "Transición / Inicio de Partida".to_string(),
    );

    // 3. Avanzar 220 frames hasta que el juego esté en plena ejecución con sprites y fondo activos
    for _ in 0..220 {
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }

    project.capture_current_live_scene(
        "En Juego — Inicio de Nivel".to_string(),
    );

    // 4. Simular automáticamente avance y acción (Derecha + Disparo) para capturar nuevos sprites/enemigos sin jugar
    for f in 0..45 {
        let btns = if (f % 8) < 5 {
            JoypadButtons::RIGHT | JoypadButtons::BUTTON_B
        } else {
            JoypadButtons::RIGHT
        };
        project.nes_live.set_controller1(btns);
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }
    project.nes_live.set_controller1(JoypadButtons::empty());
    project.capture_current_live_scene(
        "En Juego — Avance y Disparo (Auto)".to_string(),
    );

    // 5. Simular salto y movimiento adicional para capturar más poses y elementos del escenario
    for f in 0..45 {
        let btns = if (f % 10) < 6 {
            JoypadButtons::RIGHT | JoypadButtons::BUTTON_A | JoypadButtons::BUTTON_B
        } else {
            JoypadButtons::RIGHT | JoypadButtons::UP
        };
        project.nes_live.set_controller1(btns);
        project.nes_live.step_frame();
        project.nes_live.bus.apu.audio_samples.clear();
    }
    project.nes_live.set_controller1(JoypadButtons::empty());
    project.capture_current_live_scene(
        "En Juego — Salto y Exploración (Auto)".to_string(),
    );

    // Seleccionar por defecto la escena 3 (En Juego — Inicio de Nivel)
    project.active_scene_idx = 2.min(project.scenes.len().saturating_sub(1));

    // Detectar o autodescubrir de forma universal el motor sonoro del juego y sus pistas reales
    crate::runtime::jukebox_apu::detect_or_discover_audio_engine(project);

    Ok(())
}

pub fn load_asm_subroutines(asm_path: &Path, project: &mut NgsProject) -> Result<usize, String> {
    let raw_asm = fs::read_to_string(asm_path)
        .map_err(|e| format!("No se pudo leer '{}': {}", asm_path.display(), e))?;

    project.asm_file_path = Some(asm_path.to_path_buf());
    project.subroutines.clear();

    let lines: Vec<&str> = raw_asm.lines().collect();
    let mut current_category = "[GAME LOGIC]".to_string();

    for (idx, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains("[AUDIO / APU]") {
            current_category = "[AUDIO / APU]".to_string();
        } else if trimmed.contains("[VIDEO / PPU]") {
            current_category = "[VIDEO / PPU]".to_string();
        } else if trimmed.contains("[JOYPAD / INPUT]") {
            current_category = "[JOYPAD / INPUT]".to_string();
        } else if trimmed.contains("[GAME LOGIC]") || trimmed.contains("[MAPPER") {
            current_category = "[GAME LOGIC]".to_string();
        }

        if (trimmed.starts_with("sub_") || trimmed.starts_with("nmi_") || trimmed.starts_with("reset_"))
            && trimmed.ends_with(':')
        {
            let name = trimmed.trim_end_matches(':').to_string();
            let addr = name
                .split('_')
                .last()
                .and_then(|h| u16::from_str_radix(h, 16).ok())
                .unwrap_or(0x8000);

            let mut preview = Vec::new();
            for next_line in lines.iter().skip(idx + 1).take(14) {
                preview.push((*next_line).to_string());
                if next_line.contains("RTS") || next_line.contains("RTI") {
                    break;
                }
            }

            let body_joined = preview.join(" ");
            let cat = if body_joined.contains("$400") || body_joined.contains("$4015") {
                "[AUDIO / APU]".to_string()
            } else if body_joined.contains("$200") || body_joined.contains("$4014") {
                "[VIDEO / PPU]".to_string()
            } else if body_joined.contains("$4016") || body_joined.contains("$4017") {
                "[JOYPAD / INPUT]".to_string()
            } else {
                current_category.clone()
            };

            project.subroutines.push(SubroutineInfo {
                addr,
                name,
                category: cat,
                line_in_asm: idx + 1,
                preview_asm: preview,
            });
        }
    }

    Ok(project.subroutines.len())
}

fn extract_subroutines_from_c(raw_c: &str, project: &mut NgsProject) {
    let lines: Vec<&str> = raw_c.lines().collect();
    for (idx, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.starts_with("case 0x") && t.contains(':') {
            if let Some(hex) = t.strip_prefix("case 0x").and_then(|s| s.get(0..4)) {
                if let Ok(addr) = u16::from_str_radix(hex, 16) {
                    let mut preview = Vec::new();
                    let mut is_audio = false;
                    let mut is_video = false;
                    for next in lines.iter().skip(idx).take(12) {
                        preview.push((*next).trim().to_string());
                        if next.contains("0x400") || next.contains("0x4015") {
                            is_audio = true;
                        }
                        if next.contains("0x200") || next.contains("0x4014") {
                            is_video = true;
                        }
                    }
                    if is_audio || is_video || (addr & 0x00FF) == 0 {
                        let category = if is_audio {
                            "[AUDIO / APU]"
                        } else if is_video {
                            "[VIDEO / PPU]"
                        } else {
                            "[GAME LOGIC]"
                        };
                        project.subroutines.push(SubroutineInfo {
                            addr,
                            name: format!("loc_{:04X}", addr),
                            category: category.to_string(),
                            line_in_asm: idx + 1,
                            preview_asm: preview,
                        });
                    }
                }
            }
        }
    }
}

fn parse_define_usize(src: &str, prefix: &str) -> Option<usize> {
    src.lines()
        .find_map(|l| l.trim().strip_prefix(prefix).map(|r| r.trim().to_string()))
        .and_then(|v| {
            let tok = v.split_whitespace().next()?.to_string();
            match tok.strip_prefix("0x").or_else(|| tok.strip_prefix("0X")) {
                Some(h) => usize::from_str_radix(h, 16).ok(),
                None => tok.parse::<usize>().ok(),
            }
        })
}

fn parse_assignment_u8(src: &str, var_name: &str) -> Option<u8> {
    for line in src.lines() {
        let t = line.trim();
        if t.starts_with('*') || t.starts_with("//") || t.starts_with("/*") {
            continue;
        }
        if let Some(pos) = t.find(var_name) {
            let after = &t[pos + var_name.len()..];
            if let Some(eq) = after.find('=') {
                let val: String = after[eq + 1..]
                    .trim()
                    .chars()
                    .take_while(|c| c.is_ascii_hexdigit() || *c == 'x' || *c == 'X')
                    .collect();
                if let Some(hex) = val.strip_prefix("0x").or_else(|| val.strip_prefix("0X")) {
                    return u8::from_str_radix(hex, 16).ok();
                }
                return val.parse::<u8>().ok();
            }
        }
    }
    None
}

fn parse_define_u16(src: &str, prefix: &str) -> Option<u16> {
    for line in src.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix(prefix) {
            let val_str = rest.split_whitespace().next()?;
            if let Some(hex) = val_str.strip_prefix("0x").or_else(|| val_str.strip_prefix("0X")) {
                return u16::from_str_radix(hex, 16).ok();
            } else {
                return val_str.parse::<u16>().ok();
            }
        }
    }
    None
}

/// Localiza la DECLARACIÓN real del arreglo (ej. `uint8_t PRG_ROM[PRG_ROM_SIZE] = {`),
/// ignorando menciones en comentarios, y extrae sus bytes descartando los comentarios de offset `/* 0x00010 */`.
fn extract_c_byte_array(src: &str, marker: &str) -> Vec<u8> {
    let mut search_from = 0usize;
    let mut decl_pos = None;
    while let Some(rel) = src[search_from..].find(marker) {
        let abs = search_from + rel;
        let line_start = src[..abs].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line_end = src[abs..].find('\n').map(|p| abs + p).unwrap_or(src.len());
        let line = &src[line_start..line_end];
        let trimmed = line.trim_start();
        let is_comment = trimmed.starts_with('*') || trimmed.starts_with("/*") || trimmed.starts_with("//");
        if !is_comment && line.contains("uint8_t") && line.contains('=') && line.contains('{') {
            decl_pos = Some(abs);
            break;
        }
        search_from = abs + marker.len();
    }
    let Some(start_pos) = decl_pos else {
        return Vec::new();
    };
    let after_marker = &src[start_pos..];
    let Some(open_brace) = after_marker.find('{') else {
        return Vec::new();
    };
    let after_brace = &after_marker[open_brace + 1..];
    let Some(close_brace) = after_brace.find("};") else {
        return Vec::new();
    };
    let raw_body = &after_brace[..close_brace];

    // Eliminar comentarios /* ... */ (offsets de fila) para no confundirlos con bytes
    let mut body = String::with_capacity(raw_body.len());
    let mut rest = raw_body;
    while let Some(c_start) = rest.find("/*") {
        body.push_str(&rest[..c_start]);
        match rest[c_start + 2..].find("*/") {
            Some(c_end) => rest = &rest[c_start + 2 + c_end + 2..],
            None => {
                rest = "";
                break;
            }
        }
    }
    body.push_str(rest);

    let mut bytes = Vec::with_capacity(65536);
    for token in body.split(|c: char| c == ',' || c.is_whitespace()) {
        let t = token.trim();
        if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
            if let Ok(b) = u8::from_str_radix(hex, 16) {
                bytes.push(b);
            }
        }
    }
    bytes
}
