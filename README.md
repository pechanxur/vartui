# vartui

TUI de terminal para manejar registros de horario en el VAR, con CLI JSON y servidor MCP para automatizacion.

## Que hace

- Ver entradas por dia y proyecto
- Cambiar rangos rapido (mes/semana/custom)
- Crear y duplicar entradas desde la terminal
- Exponer datos/acciones via `vartui api ...`
- Exponer control del TUI via `vartui mcp` con respuestas en TOON

## Quick start

```bash
cargo build --release
./target/release/vartui
```

Para desarrollo local:

```bash
cargo run --
```

Para servidor MCP por stdio:

```bash
./target/release/vartui mcp
```

## Atajos TUI

### Modo normal

- `q` o `Ctrl+C`: salir
- `j`/`k` o `Down`/`Up`: mover seleccion
- `l`: enfocar panel de entradas
- `h` o `Esc`: volver al panel de dias
- `r`: refrescar datos
- `f`: editar rango de fechas
- `n`: nueva entrada
- `d`: duplicar entrada seleccionada
- `c`: abrir modal de config

### Formularios

- `Tab` / `Shift+Tab`: siguiente/anterior campo
- `Enter`: confirmar/seleccionar/guardar
- `Esc`: cancelar/cerrar modal
- `Ctrl+u` (config): limpiar campo actual
- `Ctrl+r` (config): restablecer configuracion
- `Up` / `Down` (campo Tema): navegar lista desplegable de temas

## Formatos de rango

- `AUTO` or `AUTO-MONTH`
- `AUTO-WEEK`
- `YYYY-MM-DD..YYYY-MM-DD`

## CLI API (JSON)

```bash
./target/release/vartui api projects --pretty
./target/release/vartui api days --range AUTO-WEEK --pretty
./target/release/vartui api entries --range AUTO-MONTH --pretty
./target/release/vartui api create-entry \
  --date 2026-02-09 \
  --project-id 123 \
  --description "Sync de producto" \
  --minutes 90 \
  --billable true
```

## MCP (TOON)

`vartui mcp` levanta un servidor MCP (stdio, JSON-RPC) independiente del subcomando `api`.

- Tools disponibles:
  - `vartui.session.create`
  - `vartui.session.snapshot`
  - `vartui.session.key`
  - `vartui.session.action` (recomendada para menor costo de tokens)
  - `vartui.session.close`
- Todas las respuestas de `tools/call` regresan `content[0].text` en formato TOON.
- `structuredContent` es opcional (`structured=true` / `stc=true`), para ahorrar tokens viene apagado por default.

### Modo low-token (recomendado)

- Usa `vartui.session.action` en lugar de enviar muchas teclas una por una.
- Usa `view=tiny` o `view=none` (`vw=t` / `vw=0`) para respuestas mas cortas.
- Usa aliases cortos en args: `sid`, `a`, `f`, `v`, `k`, `t`, `i`, `vw`, `md`, `me`, `stc`.
- Para lotes, manda `actions` con varios pasos en una sola llamada.

Ejemplo de batch minimal:

```json
{
  "name": "vartui.session.action",
  "arguments": {
    "sid": "session-1",
    "actions": [
      { "a": "oa" },
      { "a": "sf", "f": "desc", "v": "Sync semanal" },
      { "a": "sf", "f": "m", "v": "90" },
      { "a": "se" }
    ],
    "vw": "t"
  }
}
```

### Paridad con TUI

- `vartui.session.key` mantiene paridad 1:1 con el teclado del TUI.
- `vartui.session.action` agrega operaciones semanticas (y batch) para flujos largos:
  - Navegacion: `next_day`, `previous_day`, `focus_entries`, `focus_days`
  - Rango: `set_range`, `open_range_editor`, `submit_range`
  - Entrada: `open_add_entry`, `set_entry_field`, `select_project`, `submit_entry`
  - Config: `open_config`, `set_config_field` (`token`, `base_url`, `default_range`, `theme`), `save_config`
  - Fallback exacto: `send_key`

### Configuracion MCP: Claude Desktop

Archivo: `~/.claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "vartui": {
      "command": "/Users/davidsanchez/Code/encorto/vartui/target/release/vartui",
      "args": ["mcp"]
    }
  }
}
```

### Configuracion MCP: Cline (VS Code)

En `settings.json`:

```json
{
  "cline.mcpServers": {
    "vartui": {
      "command": "/Users/davidsanchez/Code/encorto/vartui/target/release/vartui",
      "args": ["mcp"]
    }
  }
}
```

## Configuration

- `VAR_TOKEN`: token de auth (requerido si no esta en config)
- `VAR_BASE_URL`: base URL del API (default: `https://var.elaniin.com/api`)
- `theme`: preset visual para toda la TUI (default: `tokyo-night`, tambien soporta `auto`)
- La config persistente se guarda con `confy` y se edita desde el modal (`c`)

### Catalogo de temas (preset)

Se usa el catalogo de `ratatui-themes` (sin crear paletas desde cero):

- `dracula`
- `one-dark-pro`
- `nord`
- `catppuccin-mocha`
- `catppuccin-latte`
- `gruvbox-dark`
- `gruvbox-light`
- `tokyo-night`
- `solarized-dark`
- `solarized-light`
- `monokai-pro`
- `rose-pine`
- `kanagawa`
- `everforest`
- `cyberpunk`

Ejemplo en config local (confy):

```toml
theme = "catppuccin-mocha"
```

Modo automatico (claro/oscuro segun sistema):

```toml
theme = "auto"
```

Notas de deteccion:

- En macOS se detecta usando `AppleInterfaceStyle` (Dark/Light).
- Si no se puede detectar, cae a `tokyo-night`.
- Puedes forzar el modo con `VARTUI_SYSTEM_THEME=dark|light`.
- En el modal de config el tema tiene preview en vivo (incluye panel de acciones) antes de guardar.

## Desarrollo

```bash
cargo fmt
cargo test
```
