use crate::graphics::metasprite::MetaspriteEntity;
use crate::graphics::{ExtendedGraphicAsset, GraphicId, GraphicSource};
use crate::project::{nes_color_rgb_u8, NgsProject};
use std::fs;
use std::path::Path;

/// Imagen RGBA decodificada lista para el Banco de Memoria Extendida Ilimitada o cuantización NES (Fase G5).
#[derive(Clone, Debug)]
pub struct DecodedRgbaImage {
    pub width: u32,
    pub height: u32,
    /// Píxeles empaquetados como `0xAABBGGRR` (formato nativo little-endian `[R, G, B, A]`).
    pub pixels: Vec<u32>,
}

impl DecodedRgbaImage {
    pub fn pack_rgba(r: u8, g: u8, b: u8, a: u8) -> u32 {
        (r as u32) | ((g as u32) << 8) | ((b as u32) << 16) | ((a as u32) << 24)
    }

    pub fn unpack_rgba(px: u32) -> [u8; 4] {
        [
            (px & 0xFF) as u8,
            ((px >> 8) & 0xFF) as u8,
            ((px >> 16) & 0xFF) as u8,
            ((px >> 24) & 0xFF) as u8,
        ]
    }

    pub fn get_pixel_clamped(&self, x: u32, y: u32) -> [u8; 4] {
        if self.pixels.is_empty() || self.width == 0 || self.height == 0 {
            return [0, 0, 0, 0];
        }
        let cx = x.min(self.width - 1) as usize;
        let cy = y.min(self.height - 1) as usize;
        Self::unpack_rgba(self.pixels[cy * (self.width as usize) + cx])
    }
}

/// Decodifica un archivo `.PNG` o `.BMP` real desde disco hacia un buffer `DecodedRgbaImage`.
pub fn load_rgba_image_from_path(path: &Path) -> Result<DecodedRgbaImage, String> {
    let bytes = fs::read(path)
        .map_err(|e| format!("No se pudo leer la imagen '{}': {}", path.display(), e))?;
    decode_rgba_from_bytes(&bytes)
}

/// Decodifica bytes `.PNG` o `.BMP` en memoria a `DecodedRgbaImage`.
pub fn decode_rgba_from_bytes(bytes: &[u8]) -> Result<DecodedRgbaImage, String> {
    if bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n" {
        decode_png_bytes(bytes)
    } else if bytes.len() >= 2 && &bytes[0..2] == b"BM" {
        decode_bmp_bytes(bytes)
    } else {
        Err("Formato de imagen no reconocido. Selecciona un archivo .PNG o .BMP válido.".to_string())
    }
}

/// Registra una imagen `DecodedRgbaImage` en el **Banco de Memoria Extendida Ilimitada**
/// vinculándola a un personaje (`MetaspriteEntity`) o como una nueva creación independiente.
pub fn register_extended_hd_asset(
    project: &mut NgsProject,
    name: String,
    target_id: Option<GraphicId>,
    image: &DecodedRgbaImage,
    source_path: Option<String>,
) -> usize {
    // Si ya existía un activo para el mismo target_id o nombre, actualizarlo
    if let Some(tid) = &target_id {
        if let Some(existing) = project
            .extended_assets
            .iter_mut()
            .find(|a| a.target_graphic_id.as_ref() == Some(tid))
        {
            existing.name = name;
            existing.width = image.width;
            existing.height = image.height;
            existing.rgba_pixels = image.pixels.clone();
            existing.source_path = source_path;
            let id = existing.asset_id;
            project.auto_sync_workspace();
            return id;
        }
    }

    let next_id = project.extended_assets.len() + 1;
    project.extended_assets.push(ExtendedGraphicAsset {
        asset_id: next_id,
        name,
        target_graphic_id: target_id,
        width: image.width,
        height: image.height,
        rgba_pixels: image.pixels.clone(),
        source_path,
    });
    project.auto_sync_workspace();
    next_id
}

/// Adapta y cuantiza automáticamente una imagen `DecodedRgbaImage` hacia los tiles `8x8`
/// de un personaje `MetaspriteEntity`, respetando su paleta activa, `FlipH`/`FlipV` y
/// actualizando automáticamente el mapa de memoria (`CHR-ROM` o `CHR-RAM` + `PRG-ROM`).
pub fn bake_rgba_into_entity_nes_tiles(
    project: &mut NgsProject,
    entity: &MetaspriteEntity,
    scene_idx: usize,
    image: &DecodedRgbaImage,
) {
    if image.width == 0 || image.height == 0 || entity.width == 0 || entity.height == 0 {
        return;
    }

    let palette_table = project
        .scenes
        .get(scene_idx)
        .map(|s| s.palette_table)
        .unwrap_or(project.nes_live.bus.ppu.palette_table);

    for cy in 0..entity.height {
        for cx in 0..entity.width {
            let src_x = ((cx as u32) * image.width) / (entity.width as u32);
            let src_y = ((cy as u32) * image.height) / (entity.height as u32);
            let [r, g, b, a] = image.get_pixel_clamped(src_x, src_y);

            let Some((vram_tile, row, col, pal_idx)) =
                entity.map_canvas_pixel_to_vram_tile(cx, cy)
            else {
                continue;
            };

            let color_idx = if a < 96 {
                0u8
            } else {
                let pal_base = 16 + ((pal_idx as usize) & 3) * 4;
                let mut best_c = 1u8;
                let mut best_dist = i32::MAX;
                for c in 1..=3u8 {
                    let nes_col = palette_table[(pal_base + (c as usize)) & 31];
                    let [pr, pg, pb] = nes_color_rgb_u8(nes_col);
                    let dr = (r as i32) - (pr as i32);
                    let dg = (g as i32) - (pg as i32);
                    let db = (b as i32) - (pb as i32);
                    let dist = dr * dr + dg * dg + db * db;
                    if dist < best_dist {
                        best_dist = dist;
                        best_c = c;
                    }
                }
                best_c
            };

            // Localizar el tile en global_tiles para esa escena y aplicar el cambio
            if let Some(global_idx) = project.global_tiles.iter().position(|t| {
                matches!(
                    t.id.source,
                    GraphicSource::OamSprite
                        | GraphicSource::ChrRamScene
                        | GraphicSource::NametableBg
                ) && t.id.bank_or_scene == scene_idx
                    && t.id.tile_index == vram_tile
            }) {
                project.set_tile_pixel(global_idx, row, col, color_idx);
            }
        }
    }
    project.auto_sync_workspace();
}

/// Exporta cualquier buffer RGBA (`width x height`) como un archivo `.PNG` estándar válido.
pub fn encode_uncompressed_png_rgba(width: u32, height: u32, pixels: &[u32]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"\x89PNG\r\n\x1a\n");

    // 1. IHDR chunk (13 bytes)
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth = 8
    ihdr.push(6); // color type = 6 (RGBA)
    ihdr.push(0); // compression = 0
    ihdr.push(0); // filter = 0
    ihdr.push(0); // interlace = 0
    write_png_chunk(&mut out, b"IHDR", &ihdr);

    // 2. Construir scanlines crudas con filtro 0 (None) al inicio de cada fila
    let row_bytes = (width as usize) * 4 + 1;
    let mut raw_data = Vec::with_capacity(row_bytes * (height as usize));
    for y in 0..(height as usize) {
        raw_data.push(0u8); // Filter type 0 (None)
        for x in 0..(width as usize) {
            let px = pixels
                .get(y * (width as usize) + x)
                .copied()
                .unwrap_or(0);
            let [r, g, b, a] = DecodedRgbaImage::unpack_rgba(px);
            raw_data.extend_from_slice(&[r, g, b, a]);
        }
    }

    // Empaquetar en stream Zlib con bloques Deflate no comprimidos (stored blocks) + Adler32
    let mut zlib = Vec::new();
    zlib.push(0x78); // CMF
    zlib.push(0x01); // FLG

    let chunks: Vec<&[u8]> = raw_data.chunks(65535).collect();
    if chunks.is_empty() {
        zlib.extend_from_slice(&[0x01, 0x00, 0x00, 0xFF, 0xFF]);
    } else {
        for (i, ch) in chunks.iter().enumerate() {
            let is_last = i + 1 == chunks.len();
            zlib.push(if is_last { 0x01 } else { 0x00 });
            let len = ch.len() as u16;
            let nlen = !len;
            zlib.extend_from_slice(&len.to_le_bytes());
            zlib.extend_from_slice(&nlen.to_le_bytes());
            zlib.extend_from_slice(ch);
        }
    }
    let adler = compute_adler32(&raw_data);
    zlib.extend_from_slice(&adler.to_be_bytes());
    write_png_chunk(&mut out, b"IDAT", &zlib);

    // 3. IEND chunk
    write_png_chunk(&mut out, b"IEND", &[]);
    out
}

fn write_png_chunk(out: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(chunk_type);
    out.extend_from_slice(data);
    let mut crc = 0xFFFF_FFFFu32;
    for &b in chunk_type.iter().chain(data.iter()) {
        crc ^= b as u32;
        for _ in 0..8 {
            if (crc & 1) != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    out.extend_from_slice(&(!crc).to_be_bytes());
}

fn compute_adler32(data: &[u8]) -> u32 {
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + (byte as u32)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

// =====================================================================
// DECODIFICADOR NATIVO DE ARCHIVOS .BMP (24-bit y 32-bit)
// =====================================================================
fn decode_bmp_bytes(bytes: &[u8]) -> Result<DecodedRgbaImage, String> {
    if bytes.len() < 54 {
        return Err("Archivo .BMP truncado o inválido.".to_string());
    }
    let data_offset = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]) as usize;
    let width = i32::from_le_bytes([bytes[18], bytes[19], bytes[20], bytes[21]]);
    let height = i32::from_le_bytes([bytes[22], bytes[23], bytes[24], bytes[25]]);
    let bpp = u16::from_le_bytes([bytes[28], bytes[29]]);

    if width <= 0 || height == 0 {
        return Err("Dimensiones .BMP inválidas.".to_string());
    }
    let w = width as u32;
    let h = height.unsigned_abs();
    let top_down = height < 0;
    let bytes_per_pixel = (bpp / 8) as usize;
    if bytes_per_pixel < 3 {
        return Err("El archivo .BMP debe ser de 24 o 32 bits.".to_string());
    }

    let row_stride = ((w as usize) * bytes_per_pixel + 3) & !3;
    let mut pixels = vec![0u32; (w * h) as usize];

    for row in 0..(h as usize) {
        let src_row = if top_down {
            row
        } else {
            (h as usize) - 1 - row
        };
        let row_start = data_offset + src_row * row_stride;
        for col in 0..(w as usize) {
            let p_off = row_start + col * bytes_per_pixel;
            if p_off + bytes_per_pixel <= bytes.len() {
                let b = bytes[p_off];
                let g = bytes[p_off + 1];
                let r = bytes[p_off + 2];
                let a = if bytes_per_pixel >= 4 {
                    bytes[p_off + 3]
                } else {
                    255
                };
                pixels[row * (w as usize) + col] = DecodedRgbaImage::pack_rgba(r, g, b, a);
            }
        }
    }

    Ok(DecodedRgbaImage {
        width: w,
        height: h,
        pixels,
    })
}

// =====================================================================
// DECODIFICADOR NATIVO DE ARCHIVOS .PNG (RFC 2083 + Zlib/Deflate RFC 1951)
// =====================================================================
fn decode_png_bytes(bytes: &[u8]) -> Result<DecodedRgbaImage, String> {
    let mut pos = 8usize;
    let mut width = 0u32;
    let mut height = 0u32;
    let mut bit_depth = 8u8;
    let mut color_type = 6u8;
    let mut palette: Vec<[u8; 3]> = Vec::new();
    let mut trns: Vec<u8> = Vec::new();
    let mut idat_compressed = Vec::new();

    while pos + 8 <= bytes.len() {
        let len =
            u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]])
                as usize;
        let ctype = &bytes[pos + 4..pos + 8];
        let data_start = pos + 8;
        let data_end = data_start.saturating_add(len);
        if data_end > bytes.len() {
            break;
        }
        let chunk_data = &bytes[data_start..data_end];

        match ctype {
            b"IHDR" if chunk_data.len() >= 13 => {
                width = u32::from_be_bytes([
                    chunk_data[0],
                    chunk_data[1],
                    chunk_data[2],
                    chunk_data[3],
                ]);
                height = u32::from_be_bytes([
                    chunk_data[4],
                    chunk_data[5],
                    chunk_data[6],
                    chunk_data[7],
                ]);
                bit_depth = chunk_data[8];
                color_type = chunk_data[9];
            }
            b"PLTE" => {
                for rgb in chunk_data.chunks_exact(3) {
                    palette.push([rgb[0], rgb[1], rgb[2]]);
                }
            }
            b"tRNS" => {
                trns.extend_from_slice(chunk_data);
            }
            b"IDAT" => {
                idat_compressed.extend_from_slice(chunk_data);
            }
            b"IEND" => break,
            _ => {}
        }
        pos = data_end.saturating_add(4); // saltar CRC
    }

    if width == 0 || height == 0 || idat_compressed.len() < 6 {
        return Err("Cabecera IHDR o datos IDAT inválidos en el archivo .PNG.".to_string());
    }
    if bit_depth != 8 {
        return Err(format!(
            "Profundidad de bits PNG ({}) no soportada directamente; usa un PNG de 8 bits por canal.",
            bit_depth
        ));
    }

    // Descomprimir stream Zlib (saltando los 2 bytes de cabecera Zlib)
    let raw_scanlines = inflate_zlib_stream(&idat_compressed)?;

    let bpp = match color_type {
        6 => 4usize, // RGBA
        2 => 3usize, // RGB
        4 => 2usize, // Gray + Alpha
        0 | 3 => 1usize, // Gray o Indexed
        _ => return Err(format!("Color type PNG ({}) no soportado.", color_type)),
    };

    let stride = (width as usize) * bpp;
    let expected_len = (stride + 1) * (height as usize);
    if raw_scanlines.len() < expected_len {
        return Err("Datos descomprimidos PNG incompletos.".to_string());
    }

    let mut unfiltered = vec![0u8; stride * (height as usize)];
    for y in 0..(height as usize) {
        let filter = raw_scanlines[y * (stride + 1)];
        let src_row = &raw_scanlines[y * (stride + 1) + 1..(y + 1) * (stride + 1)];
        let dst_start = y * stride;

        for i in 0..stride {
            let x = src_row[i];
            let a = if i >= bpp {
                unfiltered[dst_start + i - bpp]
            } else {
                0
            };
            let b = if y > 0 {
                unfiltered[(y - 1) * stride + i]
            } else {
                0
            };
            let c = if y > 0 && i >= bpp {
                unfiltered[(y - 1) * stride + i - bpp]
            } else {
                0
            };

            let recon = match filter {
                0 => x,
                1 => x.wrapping_add(a),
                2 => x.wrapping_add(b),
                3 => x.wrapping_add(((a as u16 + b as u16) / 2) as u8),
                4 => x.wrapping_add(paeth_predictor(a, b, c)),
                _ => x,
            };
            unfiltered[dst_start + i] = recon;
        }
    }

    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..(height as usize) {
        let row = &unfiltered[y * stride..(y + 1) * stride];
        for x in 0..(width as usize) {
            let (r, g, b, a) = match color_type {
                6 => (
                    row[x * 4],
                    row[x * 4 + 1],
                    row[x * 4 + 2],
                    row[x * 4 + 3],
                ),
                2 => (row[x * 3], row[x * 3 + 1], row[x * 3 + 2], 255),
                4 => (row[x * 2], row[x * 2], row[x * 2], row[x * 2 + 1]),
                0 => (row[x], row[x], row[x], 255),
                3 => {
                    let idx = row[x] as usize;
                    let [pr, pg, pb] = palette.get(idx).copied().unwrap_or([0, 0, 0]);
                    let pa = trns.get(idx).copied().unwrap_or(255);
                    (pr, pg, pb, pa)
                }
                _ => (0, 0, 0, 0),
            };
            pixels.push(DecodedRgbaImage::pack_rgba(r, g, b, a));
        }
    }

    Ok(DecodedRgbaImage {
        width,
        height,
        pixels,
    })
}

fn paeth_predictor(a: u8, b: u8, c: u8) -> u8 {
    let p = (a as i32) + (b as i32) - (c as i32);
    let pa = (p - (a as i32)).abs();
    let pb = (p - (b as i32)).abs();
    let pc = (p - (c as i32)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

struct BitReader<'a> {
    data: &'a [u8],
    byte_pos: usize,
    bit_pos: u8,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            byte_pos: 0,
            bit_pos: 0,
        }
    }

    fn read_bit(&mut self) -> Result<u32, String> {
        if self.byte_pos >= self.data.len() {
            return Err("Fin inesperado del stream Deflate.".to_string());
        }
        let bit = ((self.data[self.byte_pos] >> self.bit_pos) & 1) as u32;
        self.bit_pos += 1;
        if self.bit_pos == 8 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
        Ok(bit)
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, String> {
        let mut val = 0u32;
        for i in 0..count {
            val |= self.read_bit()? << i;
        }
        Ok(val)
    }

    fn align_byte(&mut self) {
        if self.bit_pos != 0 {
            self.bit_pos = 0;
            self.byte_pos += 1;
        }
    }

    fn read_u16_le(&mut self) -> Result<u16, String> {
        self.align_byte();
        if self.byte_pos + 2 > self.data.len() {
            return Err("Fin inesperado leyendo u16 en bloque Deflate.".to_string());
        }
        let v = u16::from_le_bytes([self.data[self.byte_pos], self.data[self.byte_pos + 1]]);
        self.byte_pos += 2;
        Ok(v)
    }
}

struct HuffmanTree {
    counts: [u16; 16],
    symbols: Vec<u16>,
}

impl HuffmanTree {
    fn from_code_lengths(lengths: &[u8]) -> Self {
        let mut counts = [0u16; 16];
        for &l in lengths {
            if (l as usize) > 0 && (l as usize) < 16 {
                counts[l as usize] += 1;
            }
        }
        let mut offsets = [0u16; 16];
        let mut total = 0u16;
        for bits in 1..16 {
            offsets[bits] = total;
            total += counts[bits];
        }
        let mut symbols = vec![0u16; total as usize];
        for (sym, &l) in lengths.iter().enumerate() {
            let len = l as usize;
            if len > 0 && len < 16 {
                let idx = offsets[len] as usize;
                symbols[idx] = sym as u16;
                offsets[len] += 1;
            }
        }
        Self { counts, symbols }
    }

    fn decode(&self, reader: &mut BitReader) -> Result<u16, String> {
        let mut code = 0i32;
        let mut first = 0i32;
        let mut index = 0i32;
        for len in 1..16 {
            code |= reader.read_bit()? as i32;
            let count = self.counts[len] as i32;
            if code - count < first {
                return Ok(self.symbols[(index + (code - first)) as usize]);
            }
            index += count;
            first += count;
            first <<= 1;
            code <<= 1;
        }
        Err("Código Huffman inválido en stream PNG.".to_string())
    }
}

fn inflate_zlib_stream(zlib: &[u8]) -> Result<Vec<u8>, String> {
    if zlib.len() < 6 {
        return Err("Stream Zlib demasiado corto.".to_string());
    }
    let fdict = (zlib[1] & 0x20) != 0;
    let start = if fdict { 6 } else { 2 };
    let mut reader = BitReader::new(&zlib[start..]);
    let mut out = Vec::new();

    loop {
        let bfinal = reader.read_bit()?;
        let btype = reader.read_bits(2)?;

        match btype {
            0 => {
                let len = reader.read_u16_le()? as usize;
                let _nlen = reader.read_u16_le()?;
                if reader.byte_pos + len > reader.data.len() {
                    return Err("Bloque no comprimido excede el tamaño del stream.".to_string());
                }
                out.extend_from_slice(&reader.data[reader.byte_pos..reader.byte_pos + len]);
                reader.byte_pos += len;
            }
            1 | 2 => {
                let (lit_tree, dist_tree) = if btype == 1 {
                    let mut lit_lens = vec![0u8; 288];
                    for i in 0..=143 {
                        lit_lens[i] = 8;
                    }
                    for i in 144..=255 {
                        lit_lens[i] = 9;
                    }
                    for i in 256..=279 {
                        lit_lens[i] = 7;
                    }
                    for i in 280..=287 {
                        lit_lens[i] = 8;
                    }
                    let dist_lens = vec![5u8; 32];
                    (
                        HuffmanTree::from_code_lengths(&lit_lens),
                        HuffmanTree::from_code_lengths(&dist_lens),
                    )
                } else {
                    let hlit = (reader.read_bits(5)? as usize) + 257;
                    let hdist = (reader.read_bits(5)? as usize) + 1;
                    let hclen = (reader.read_bits(4)? as usize) + 4;
                    const CL_ORDER: [usize; 19] = [
                        16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
                    ];
                    let mut cl_lens = [0u8; 19];
                    for i in 0..hclen {
                        cl_lens[CL_ORDER[i]] = reader.read_bits(3)? as u8;
                    }
                    let cl_tree = HuffmanTree::from_code_lengths(&cl_lens);
                    let mut all_lens = Vec::with_capacity(hlit + hdist);
                    while all_lens.len() < hlit + hdist {
                        let sym = cl_tree.decode(&mut reader)?;
                        match sym {
                            0..=15 => all_lens.push(sym as u8),
                            16 => {
                                let prev = *all_lens.last().unwrap_or(&0);
                                let repeat = (reader.read_bits(2)? as usize) + 3;
                                for _ in 0..repeat {
                                    all_lens.push(prev);
                                }
                            }
                            17 => {
                                let repeat = (reader.read_bits(3)? as usize) + 3;
                                for _ in 0..repeat {
                                    all_lens.push(0);
                                }
                            }
                            18 => {
                                let repeat = (reader.read_bits(7)? as usize) + 11;
                                for _ in 0..repeat {
                                    all_lens.push(0);
                                }
                            }
                            _ => return Err("Símbolo de longitud Huffman inválido.".to_string()),
                        }
                    }
                    let lit_t = HuffmanTree::from_code_lengths(&all_lens[0..hlit]);
                    let dist_t = HuffmanTree::from_code_lengths(&all_lens[hlit..]);
                    (lit_t, dist_t)
                };

                const LEN_BASE: [usize; 29] = [
                    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67,
                    83, 99, 115, 131, 163, 195, 227, 258,
                ];
                const LEN_EXTRA: [u8; 29] = [
                    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5,
                    5, 5, 0,
                ];
                const DIST_BASE: [usize; 30] = [
                    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769,
                    1025, 1537, 2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
                ];
                const DIST_EXTRA: [u8; 30] = [
                    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11,
                    11, 12, 12, 13, 13,
                ];

                loop {
                    let sym = lit_tree.decode(&mut reader)?;
                    if sym < 256 {
                        out.push(sym as u8);
                    } else if sym == 256 {
                        break;
                    } else {
                        let len_idx = (sym as usize) - 257;
                        if len_idx >= 29 {
                            return Err("Código de longitud Deflate fuera de rango.".to_string());
                        }
                        let length =
                            LEN_BASE[len_idx] + (reader.read_bits(LEN_EXTRA[len_idx])? as usize);
                        let dist_sym = dist_tree.decode(&mut reader)? as usize;
                        if dist_sym >= 30 {
                            return Err("Código de distancia Deflate fuera de rango.".to_string());
                        }
                        let distance = DIST_BASE[dist_sym]
                            + (reader.read_bits(DIST_EXTRA[dist_sym])? as usize);
                        if distance == 0 || distance > out.len() {
                            return Err("Distancia LZ77 inválida en PNG.".to_string());
                        }
                        for _ in 0..length {
                            let b = out[out.len() - distance];
                            out.push(b);
                        }
                    }
                }
            }
            _ => return Err("Tipo de bloque Deflate inválido.".to_string()),
        }

        if bfinal != 0 {
            break;
        }
    }

    Ok(out)
}
