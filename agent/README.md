# Agente local (Rust)

Binario único para Windows y Linux que detecta el hardware con precisión y lo expone
a la web mediante una API local.

```sh
cargo run                      # API en http://127.0.0.1:47800 y abre la web para vincularla
cargo run -- --no-browser      # sin abrir el navegador
cargo run -- --print           # imprime el informe en JSON y sale
cargo run -- --port 47801      # otro puerto
cargo run -- --allow-origin https://mi-dominio.com   # origen extra (repetible)
cargo test
```

## Vinculación con la web

Al arrancar, el agente genera (la primera vez) un token secreto en `~/.ail-helper/token` y abre
`https://amenur.github.io/AIL_Helper/vincular#token=…`. La web lo guarda en ese navegador y lo envía
en `Authorization: Bearer …`. El token va en el fragmento (`#`), que el navegador nunca envía a un servidor.
Si el navegador no se abre, el enlace aparece en la consola del agente y se puede pegar en la web.

## API

| Método | Ruta | Token | Respuesta |
|---|---|---|---|
| GET | `/v1/health` | — | `{ status, version, install: true }` |
| GET | `/v1/hardware` | — | Informe con el formato de [`shared/schema/hardware-report.schema.json`](../shared/schema/hardware-report.schema.json) |
| GET | `/v1/pair/check` | Sí | `{ paired: true }` |
| POST | `/v1/install` | Sí | Inicia la instalación: `{ modelId, quant, context, gpuLayers }` → 202 |
| GET | `/v1/status` | Sí | Pasos con su progreso, error si lo hay, y si llama-server y Open WebUI están en marcha |
| POST | `/v1/stop` | Sí | Para llama-server y Open WebUI |

## Instalación con un clic

`POST /v1/install` ejecuta estos pasos, saltando los que ya estén hechos:

1. **llama.cpp**: última release oficial; build según la GPU (CUDA/HIP/Vulkan en Windows, Vulkan en Linux, CPU si no hay GPU). SHA-256 comprobado con el `digest` de GitHub.
2. **Modelo**: el GGUF de la cuantización pedida desde Hugging Face (también modelos partidos en varios ficheros), con reanudación y SHA-256.
3. **uv**: última release oficial.
4. **Open WebUI**: `uv tool install --python 3.11 open-webui`, con uv, Python y la caché aislados en `~/.ail-helper/uv`.
5. **Arranque**: `llama-server` en `127.0.0.1:8080` y Open WebUI en `127.0.0.1:3000`, en modo de un solo usuario (sin login) y sin las herramientas integradas (añaden miles de tokens a cada mensaje). Se comprueba que ambos respondan antes de dar el chat por listo.

Todo queda en `~/.ail-helper` (`%USERPROFILE%\.ail-helper` en Windows). Los registros de los servicios están en `~/.ail-helper/logs`.
Al cerrar el agente se paran los dos servicios.

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
- Las acciones (instalar, parar) exigen el token de emparejamiento, comparado en tiempo constante.
- Lista cerrada de operaciones: solo modelos y cuantizaciones del catálogo embebido en el binario,
  contexto y capas en rango, y campos desconocidos rechazados. Nunca ejecuta comandos que vengan de la web.
- Descargas solo desde las releases oficiales y Hugging Face, verificadas con SHA-256.

## Publicar una versión

Crear y subir una etiqueta `agent-vX.Y.Z` lanza `.github/workflows/release-agent.yml`, que compila
los binarios de Windows y Linux y los publica en GitHub Releases con su `.sha256`.
