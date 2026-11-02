# NesGameStudio — IDE Visual de Edición, Modding y Remasterización HD para Juegos NES en C

**`NesGameStudio`** es un entorno de desarrollo visual (IDE) independiente diseñado para cargar los archivos **`.c`** y **`.asm`** generados por el recompilador estático de [**`NesEmuRust`**](https://github.com/iztaneo/NesEmuRust).

Su propósito es permitir **visualizar todos los gráficos del juego, reproducir aisladamente toda su música y efectos de sonido, editar pantallas, crear niveles nuevos y modernizar el juego con gráficos HD de 32 bits y audio `.MP3`/`.WAV`**, inyectando todos los cambios **automáticamente en el código C** sin que el usuario tenga que saber programar en C ni en ensamblador 6502.

---

## Arquitectura de Lenguaje Base: **Híbrida (Editor en Rust + Juegos Generados en C)**

| Componente | Lenguaje Base | Por qué se eligió |
| :--- | :--- | :--- |
| **IDE / Estudio Visual (`NesGameStudio`)** | **Rust** (`eframe`/`egui`, `cpal`, `image`, `serde`) | Reutiliza la PPU/APU de `NesEmuRust` para la Vista Previa a 60 FPS y la Rockola Aislada, procesa código `.c`/`.asm` masivo sin errores de memoria y ofrece una GUI rápida y moderna. |
| **Motor de los Juegos Exportados (`Target Engine`)** | **C Puro (`C99/C11`)** (`Win32` + `GDI` + `WinMM`) | Compila en **~1 segundo** con `cl.exe /O2` (`MSVC`), `gcc` o `clang`, generando ejecutables nativos `.exe` independientes y ultraligeros sin requerir librerías externas. |

---

## Arquitectura General de `NesGameStudio`

```mermaid
flowchart TD
    subgraph Entrada["1. Archivos de Entrada (Exportados por NesEmuRust)"]
        CSRC["Código Fuente Base (Juego.c)"]
        ASMSRC["Desensamblado Comentado (Juego.asm)"]
    end

    subgraph NucleoStudio["2. Núcleo de NesGameStudio (Aplicación GUI en Rust + egui)"]
        PARSER["Analizador Estructural de Juego.c y Juego.asm (PRG_ROM / CHR_MEM / Rutinas)"]
        PREVIEW["Motor de Ejecución y Sondeo Aislado (Video 60 FPS + Motor APU Independiente)"]
        PROJ["Proyecto de Estudio (Juego.ngs.json + Carpeta assets/)"]
    end

    subgraph Modulos["3. Los 4 Módulos Visuales de Edición"]
        M_GFX["Módulo 1: Galería Total de Gráficos y Estudio HD (CHR + Sprites 32-bit RGBA)"]
        M_LVL["Módulo 2: Editor de Pantallas, Mapas y Nuevos Niveles en C"]
        M_AUD["Módulo 3: Rockola Aislada de Música/SFX + Inyector .MP3 / .WAV"]
        M_RAM["Módulo 4: Buscador de Variables RAM + Modificador Visual de Reglas"]
    end

    subgraph Salida["4. Inyector Automático C y Compilador Nativo"]
        INJECT["Cirujano de Código C (Inyecta Hooks HD, Audio, Niveles y Reglas automáticamente)"]
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

    INJECT -->|"Reescribe Juego_Mod.c automáticamente"| COMPILER
    COMPILER --> FINAL_EXE
```

---

## Los 4 Módulos Principales

| Módulo | Función Principal | Qué Hace Automáticamente en el `.C` |
| :--- | :--- | :--- |
| **1. Galería Total de Gráficos y Widescreen (`🎨 Todos los Sprites, HD Packs & 16:9`)** | Muestra **todos los bancos gráficos del juego** decodificados en pantalla, arma los personajes completos (*metasprites*), permite reemplazarlos por imágenes **`.PNG` / `.BMP` HD de 32 bits (16.7M colores)** y activa **Widescreen 16:9 Real (`426×240` / `854×480` / `1280×720`) o 21:9 sin estirar**. | Amplía el `FRAMEBUFFER` a Widescreen/HD, renderiza los márgenes laterales desde las *NameTables* adyacentes y capas *Parallax* (centrando el HUD) e inyecta el renderizador de texturas de 32 bits en `ppu_render_scanline()`. |
| **2. Editor de Pantallas, Parallax y Niveles (`🗺️ Escenarios, Capas & Niveles C`)** | Lienzo visual tipo *Tiled* para editar pantallas existentes (`NameTables`), agregar **múltiples capas de profundidad (*Parallax Scrolling* traseras y *Foreground* delanteras con transparencia alfa)**, arrastrar nuevos enemigos (*Overlay*) o diseñar **niveles nuevos completos**. | Genera el motor de composición multicapa (`0.1x..1.5x` + *alpha blending*), estructuras de entidades (`struct CustomEntity`) y la función `run_custom_stage_c()` dentro del `.c`. |
| **3. Rockola Aislada y Estudio de Audio (`🎵 Sound Test + .MP3 / .WAV`)** | **Reproduce aisladamente cada canción y efecto de sonido del juego** (*Sound Test / Jukebox*) con control *Solo/Mute* por canal, exporta pistas a `.WAV` y permite reemplazarlas con un clic por archivos **`.MP3`** o **`.WAV`**. | Inyecta el interceptor de pistas en la APU/RAM, silencia la melodía 8-bit reemplazada y conecta `mciSendStringA` (`.mp3`) y el mezclador `.wav` a `44.1 kHz`. |
| **4. Buscador RAM y Reglas (`🔍 Variables & Trucos Visuales`)** | Escáner en vivo para descubrir qué casilla de `RAM` guarda vidas, armas o coordenadas `(X, Y)` y modificar sus reglas mediante menús visuales (con visor `.ASM` <-> `.C` opcional). | Escribe los `#define` con nombres legibles y agrega la lógica de modificación de variables en el bucle principal del `.c`. |
| **5. Creador de Juegos Nuevos (`🚀 Motor Nativo Sin 6502, Colisiones, VFX, IA, Scripts & Consolas`)** | Funciona como un **"Mini-Unity" 2D/2.5D con bucle propio en C (cero 6502)**: permite crear juegos desde cero con **Hitboxes/Colisiones** (`SOLID`, `ONE_WAY`), **Transparencias y VFX** (*Alpha/Additive Blending*, partículas), **IA de enemigos/jefes**, **Scripts modulares** (`on_spawn`, `on_update`, `on_collision`) y exportación multiplataforma. | Mantiene `main.c` intacto, ejecuta el bucle nativo `ngs_engine_step_frame()`, traduce cada personaje/nivel a `scripts/*.h` y exporta con `engine/ngs_platform.h` hacia **Windows, Xbox Series X\|S, PlayStation 5, Switch y WebAssembly**. |

---

## Flujo de Trabajo del Usuario (Sin Tocar Código)

```mermaid
sequenceDiagram
    participant User as Creador / Modder
    participant Studio as NesGameStudio (GUI Visual)
    participant Jukebox as Galería Gráfica + Rockola Aislada
    participant Codegen as Inyector Automático C + Compilador x64

    User->>Studio: 1. Abre Juego.c y Juego.asm (generados por NesEmuRust)
    Studio->>Jukebox: 2. Extrae y muestra TODOS los gráficos y lista todas las canciones/efectos
    User->>Jukebox: 3. Escucha aisladamente las canciones y explora los sprites de cada nivel
    User->>Studio: 4. Reemplaza sprites por imágenes HD (.PNG/.BMP), asigna .MP3/.WAV y edita niveles
    Studio->>Codegen: 5. Modifica automáticamente el archivo .c e inyecta todos los módulos nuevos
    Codegen-->>User: 6. Compila y lanza Juego_Remaster_x64.exe autónomo con 1 clic
```

---

## Documentación del Plan de Implementación

Consulta el diseño detallado de cada módulo, la arquitectura de la Rockola Aislada y de la Galería Completa de Gráficos, la estructura de archivos y las 5 fases de desarrollo en **[`PLAN_IMPLEMENTACION.md`](PLAN_IMPLEMENTACION.md)**.
