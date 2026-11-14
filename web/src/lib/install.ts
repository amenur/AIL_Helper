import type { Estimate } from './estimate.ts';
import type { Gpu, Model } from './types.ts';

export type OsFamily = 'windows' | 'linux';

export interface InstallPlan {
  backend: 'cuda' | 'rocm' | 'vulkan' | 'cpu';
  backendLabel: string;
  /** Patrón del binario precompilado en las releases de ggml-org/llama.cpp. */
  llamaAsset: string;
  backendNote?: string;
  gpuLayers: number;
  llamaCommand: string;
  uvInstall: string;
  openWebUiCommand: string;
  chatUrl: string;
}

const LLAMA_PORT = 8080;
const WEBUI_PORT = 3000;

export function guessOs(): OsFamily {
  const ua = globalThis.navigator?.userAgent ?? '';
  return /Windows/i.test(ua) ? 'windows' : 'linux';
}

function pickBackend(gpu: Gpu | null, os: OsFamily): Pick<InstallPlan, 'backend' | 'backendLabel' | 'llamaAsset' | 'backendNote'> {
  if (!gpu || gpu.vendor === 'apple') {
    return {
      backend: 'cpu',
      backendLabel: 'CPU',
      llamaAsset: os === 'windows' ? 'llama-b*-bin-win-cpu-x64.zip' : 'llama-b*-bin-ubuntu-x64.zip',
      backendNote: gpu?.vendor === 'apple' ? 'macOS aún no está soportado por el agente; en Mac usa la build Metal oficial.' : undefined,
    };
  }
  if (gpu.vendor === 'nvidia') {
    return os === 'windows'
      ? { backend: 'cuda', backendLabel: 'CUDA', llamaAsset: 'llama-b*-bin-win-cuda-12.4-x64.zip + cudart-llama-bin-win-cuda-12.4-x64.zip' }
      : {
          backend: 'vulkan',
          backendLabel: 'Vulkan',
          llamaAsset: 'llama-b*-bin-ubuntu-vulkan-x64.zip',
          backendNote: 'En Linux no hay binario CUDA oficial: el agente usará Vulkan o compilará con CUDA si está instalado el toolkit.',
        };
  }
  if (gpu.vendor === 'amd') {
    return os === 'windows'
      ? { backend: 'rocm', backendLabel: 'HIP (ROCm)', llamaAsset: 'llama-b*-bin-win-hip-radeon-x64.zip', backendNote: 'Si tu Radeon no está soportada por HIP, el agente usará Vulkan.' }
      : { backend: 'vulkan', backendLabel: 'Vulkan', llamaAsset: 'llama-b*-bin-ubuntu-vulkan-x64.zip' };
  }
  return {
    backend: 'vulkan',
    backendLabel: 'Vulkan',
    llamaAsset: os === 'windows' ? 'llama-b*-bin-win-vulkan-x64.zip' : 'llama-b*-bin-ubuntu-vulkan-x64.zip',
  };
}

export function buildInstallPlan(model: Model, est: Estimate, gpu: Gpu | null, os: OsFamily): InstallPlan {
  const backend = pickBackend(gpu, os);
  const gpuLayers =
    backend.backend === 'cpu' ? 0 : est.gpuFraction >= 1 ? 999 : Math.floor(est.gpuFraction * model.layers);

  const exe = os === 'windows' ? '.\\llama-server.exe' : './llama-server';
  const llamaCommand = [
    exe,
    `-hf ${model.hfRepo}:${est.quant}`,
    `-c ${est.context}`,
    `-ngl ${gpuLayers}`,
    '--jinja',
    `--host 127.0.0.1 --port ${LLAMA_PORT}`,
  ].join(' ');

  const api = `http://127.0.0.1:${LLAMA_PORT}/v1`;
  const uvInstall =
    os === 'windows'
      ? 'powershell -ExecutionPolicy ByPass -c "irm https://astral.sh/uv/install.ps1 | iex"'
      : 'curl -LsSf https://astral.sh/uv/install.sh | sh';
  const openWebUiCommand =
    os === 'windows'
      ? [
          `$env:DATA_DIR="$HOME\\.open-webui"`,
          `$env:OPENAI_API_BASE_URL="${api}"`,
          `$env:OPENAI_API_KEY="none"`,
          `$env:ENABLE_OLLAMA_API="false"`,
          `uvx --python 3.11 open-webui@latest serve --port ${WEBUI_PORT}`,
        ].join('; ')
      : `DATA_DIR=~/.open-webui OPENAI_API_BASE_URL=${api} OPENAI_API_KEY=none ENABLE_OLLAMA_API=false \\\n  uvx --python 3.11 open-webui@latest serve --port ${WEBUI_PORT}`;

  return {
    ...backend,
    gpuLayers,
    llamaCommand,
    uvInstall,
    openWebUiCommand,
    chatUrl: `http://localhost:${WEBUI_PORT}`,
  };
}
