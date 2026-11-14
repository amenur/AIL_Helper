# Agente local (Rust)

Pendiente (fases 2 y 3). Binario único para Windows y Linux que:

- Escucha **solo** en `127.0.0.1:47800`, con CORS restringido al dominio de la web y token de emparejamiento.
- `GET /v1/hardware` → informe con el formato de [`shared/schema/hardware-report.schema.json`](../shared/schema/hardware-report.schema.json).
- Descarga y verifica el binario de llama.cpp adecuado (CUDA/HIP/Vulkan/CPU), el modelo GGUF de Hugging Face y Open WebUI (vía `uv`).
- Arranca y supervisa `llama-server` y Open WebUI.
- Solo expone acciones cerradas: nunca ejecuta comandos arbitrarios.

Crates previstos: `tokio`, `axum`, `sysinfo`, `nvml-wrapper`, `wgpu`, `windows` (DXGI), `reqwest`, `hf-hub`, `sha2`.
