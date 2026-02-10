# vartui

TUI de terminal para manejar registros de horario en el VAR, con una CLI JSON para automatizacion.

## Que hace

- Ver entradas por dia y proyecto
- Cambiar rangos rapido (mes/semana/custom)
- Crear y duplicar entradas desde la terminal
- Exponer datos/acciones via `vartui api ...`

## Quick start

```bash
cargo build --release
./target/release/vartui
```

Para desarrollo local:

```bash
cargo run --
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

## Configuration

- `VAR_TOKEN`: token de auth (requerido si no esta en config)
- `VAR_BASE_URL`: base URL del API (default: `https://var.elaniin.com/api`)
- La config persistente se guarda con `confy` y se edita desde el modal (`c`)

## Desarrollo

```bash
cargo fmt
cargo test
```
