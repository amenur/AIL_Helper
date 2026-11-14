# Agente local (Rust)

Binario único para Windows y Linux que detecta el hardware con precisión y lo expone
a la web mediante una API local.

```sh
cargo run                      # API en http://127.0.0.1:47800
cargo run -- --print           # imprime el informe en JSON y sale
cargo run -- --port 47801      # otro puerto
cargo run -- --allow-origin https://mi-dominio.com   # origen extra (repetible)
cargo test
```

## API

| Método | Ruta | Respuesta |
|---|---|---|
| GET | `/v1/health` | `{ "status": "ok", "version": "…" }` |
| GET | `/v1/hardware` | Informe con el formato de [`shared/schema/hardware-report.schema.json`](../shared/schema/hardware-report.schema.json) |

## Qué detecta

| Dato | Windows | Linux |
|---|---|---|
| GPU NVIDIA: VRAM total/libre, driver, CUDA, compute capability | NVML | NVML |
| GPU AMD/Intel: nombre, VRAM dedicada | DXGI | `/sys/class/drm` (amdgpu) + `lspci` |
| CPU: nombre, núcleos, AVX2/AVX-512… | sysinfo + `is_x86_feature_detected!` | ídem |
| RAM: total, disponible | sysinfo | sysinfo |
| RAM: tipo (DDR4/DDR5) y velocidad | WMI (`Win32_PhysicalMemory`) | SMBIOS en `/sys/firmware/dmi` (requiere root; si no, `unknown`) |
| Disco libre en la carpeta de usuario | sysinfo | sysinfo |

Las GPUs integradas se marcan con `integrated: true` y no cuentan como VRAM.

## Seguridad

- Escucha **solo** en `127.0.0.1`.
- CORS restringido a `https://amenur.github.io` y `localhost:4321` (desarrollo).
- Rechaza peticiones cuyo `Host` no sea `127.0.0.1:<puerto>` o `localhost:<puerto>` (protección contra DNS rebinding).
- Responde a la preflight de Private Network Access de Chrome.
- Solo endpoints de lectura. Las acciones de instalación llegarán con un token de emparejamiento
  y una lista cerrada de operaciones; nunca se ejecutarán comandos arbitrarios.

## Publicar una versión

Crear y subir una etiqueta `agent-vX.Y.Z` lanza `.github/workflows/release-agent.yml`, que compila
los binarios de Windows y Linux y los publica en GitHub Releases con su `.sha256`.
