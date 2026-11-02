# NesGameStudio — IDE Visual de Edición, Modding y Remasterización HD para Juegos NES en C

**`NesGameStudio`** es un entorno de desarrollo visual (IDE) independiente diseñado para cargar los archivos **`.c`** y **`.asm`** generados por el recompilador estático de [**`NesEmuRust`**](https://github.com/iztaneo/NesEmuRust).

Su propósito es permitir **visualizar, editar, modernizar y expandir** cualquier juego clásico de NES traducido a C (cambiando sprites a alta definición de 32 bits, editando pantallas, creando niveles nuevos en C puro, inyectando música `.MP3` / efectos `.WAV` y modificando reglas de juego) **sin tener que programar en ensamblador 6502 ni editar arreglos hexadecimales a mano**.

---

## Arquitectura General de `NesGameStudio`

```mermaid
flowchart TD
    subgraph Entrada["1. Archivos de Entrada (Exportados por NesEmuRust)"]
        CSRC["Código Fuente Base (Juego.c)"]
        ASMSRC["Desensamblado Comentado (Juego.asm)"]
    end

    subgraph NucleoStudio["2. Núcleo de NesGameStudio (Aplicación GUI en Rust + egui)"]
        PARSER["Analizador Estructural de Juego.c y Juego.asm"]
        PREVIEW["Motor de Ejecución en Vivo (Sandbox Preview a 60 FPS)"]
        PROJ["Proyecto de Estudio (Juego.ngs.json + Carpeta assets/)"]
    end

    subgraph Modulos["3. Los 4 Módulos Visuales de Edición"]
        M_GFX["Módulo 1: Estudio Gráfico (CHR 8x8 + Sprites HD 32-bit RGBA)"]
        M_LVL["Módulo 2: Editor de Pantallas, Mapas y Nuevos Niveles en C"]
        M_AUD["Módulo 3: Estudio de Audio (Mezclador APU + Inyector .MP3 / .WAV)"]
        M_RAM["Módulo 4: Buscador de Variables RAM + Visor Sincronizado .ASM <-> .C"]
    end

    subgraph Salida["4. Generador de Port Remasterizado"]
        INJECT["Inyector de Código C (Hooks de Video HD, Audio, Niveles y Reglas)"]
        COMPILER["Compilador Nativo (MSVC cl.exe / GCC / Clang)"]
        FINAL_EXE["Juego Remasterizado Independiente (Juego_Remaster_x64.exe + assets/)"]
    end

    CSRC --> PARSER
    ASMSRC --> PARSER
    PARSER <--> PREVIEW
    PARSER <--> PROJ

    PREVIEW <--> M_GFX
    PREVIEW <--> M_LVL
    PREVIEW <--> M_AUD
    PREVIEW <--> M_RAM

    M_GFX --> INJECT
    M_LVL --> INJECT
    M_AUD --> INJECT
    M_RAM --> INJECT

    INJECT -->|"Genera Juego_Mod.c"| COMPILER
    COMPILER --> FINAL_EXE
```

---

## Los 4 Módulos Principales

| Módulo | Función Principal | Qué Evita Hacer a Mano |
| :--- | :--- | :--- |
| **1. Estudio Gráfico (`🎨 Sprites & HD Packs`)** | Decodifica `CHR_MEM` y `OAM` en vivo como mapas de bits visibles, permite pintar píxeles, exportar/importar `.BMP` y reemplazar tiles de `8×8` (4 colores) por **Sprites HD (`16×16` / `32×32` RGBA de 32 bits)**. | Editar bytes hexadecimales (`0x3C, 0x7E...`) o lidiar con planos de 2 bits. |
| **2. Editor de Pantallas y Niveles (`🗺️ Escenarios & Niveles C`)** | Lienzo visual tipo *Tiled* para editar pantallas existentes (`NameTables`), superponer nuevas entidades/enemigos en C (*Overlay*) o diseñar **niveles nuevos 100% en C**. | Descifrar punteros comprimidos de mapas en `PRG_ROM` o límites de 2 KB de RAM. |
| **3. Estudio de Audio (`🎵 APU + .MP3 / .WAV`)** | Silencia selectivamente canales 8-bit de la APU y asocia pistas **`.MP3`** (música de fondo) y efectos **`.WAV`** polifónicos a eventos o cambios de `RAM`. | Programar llamadas a `winmm.lib` o rastrear rutinas de sonido en ensamblador. |
| **4. Inspector RAM y Visor `.ASM` <-> `.C` (`🔍 Símbolos & Reglas`)** | Escáner de memoria en vivo para descubrir qué casilla de `RAM` guarda vidas, armas o coordenadas `(X, Y)`, renombrarlas a `#define` legibles y cruzar cada instrucción `.asm` con su línea en `.c`. | Leer miles de líneas de ensamblador 6502 para encontrar una variable. |

---

## Flujo de Trabajo de Edición e Inyección

```mermaid
sequenceDiagram
    participant User as Creador / Modder
    participant Studio as NesGameStudio (GUI)
    participant Preview as Vista Previa en Vivo (60 FPS)
    participant Codegen as Inyector C + Compilador x64

    User->>Studio: 1. Abre Juego.c y Juego.asm (generados por NesEmuRust)
    Studio->>Preview: 2. Ejecuta el juego en vivo y captura RAM, CHR_MEM, OAM y NameTables
    User->>Studio: 3. Edita Sprites (.BMP / HD 32-bit), Pantallas, Audio (.MP3/.WAV) y Variables RAM
    Studio->>Codegen: 4. Inyecta Hooks en C (ppu_render_hd, apu_mix_wav, run_custom_stage_c)
    Codegen-->>User: 5. Compila y lanza Juego_Remaster_x64.exe autónomo
```

---

## Documentación del Plan de Implementación

Consulta el diseño detallado de cada módulo, la estructura de archivos y las 5 fases de desarrollo en **[`PLAN_IMPLEMENTACION.md`](PLAN_IMPLEMENTACION.md)**.
