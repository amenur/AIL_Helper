import { AGENT_URL } from './hardware.ts';

/** Token de emparejamiento con el agente local (solo en este navegador). */
const TOKEN_KEY = 'ail-helper:agent-token';

export function getToken(): string | null {
  try {
    return globalThis.localStorage?.getItem(TOKEN_KEY) ?? null;
  } catch {
    return null;
  }
}

export function setToken(token: string | null) {
  try {
    if (token) localStorage.setItem(TOKEN_KEY, token);
    else localStorage.removeItem(TOKEN_KEY);
  } catch {
    /* sin almacenamiento: habrá que vincular en cada visita */
  }
}

export const isValidToken = (t: string) => /^[0-9a-f]{64}$/.test(t);

export class AgentError extends Error {
  constructor(
    message: string,
    public status: number,
  ) {
    super(message);
  }
}

async function call<T>(method: 'GET' | 'POST', path: string, body?: unknown, timeoutMs = 4000): Promise<T> {
  const token = getToken();
  let res: Response;
  try {
    res = await fetch(`${AGENT_URL}${path}`, {
      method,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(body ? { 'Content-Type': 'application/json' } : {}),
      },
      body: body ? JSON.stringify(body) : undefined,
      signal: AbortSignal.timeout(timeoutMs),
    });
  } catch {
    throw new AgentError('No se puede conectar con el agente local', 0);
  }
  const data = await res.json().catch(() => ({}));
  if (!res.ok) throw new AgentError((data as { error?: string }).error ?? `Error ${res.status}`, res.status);
  return data as T;
}

export interface AgentHealth {
  status: string;
  version: string;
  /** Solo las versiones con instalación con un clic. */
  install?: boolean;
}

export async function health(): Promise<AgentHealth | null> {
  try {
    return await call<AgentHealth>('GET', '/v1/health', undefined, 800);
  } catch {
    return null;
  }
}

/** true si el token guardado es el del agente en marcha. */
export async function isPaired(): Promise<boolean> {
  if (!getToken()) return false;
  try {
    await call('GET', '/v1/pair/check');
    return true;
  } catch {
    return false;
  }
}

export type StepState = 'pending' | 'running' | 'done' | 'skipped' | 'error';

export interface Step {
  id: string;
  label: string;
  state: StepState;
  detail: string;
  doneBytes: number;
  totalBytes: number | null;
}

export interface AgentStatus {
  job: {
    phase: 'idle' | 'running' | 'ready' | 'error';
    modelId: string | null;
    quant: string | null;
    steps: Step[];
    error: string | null;
    chatUrl: string | null;
    llamaBackend: string | null;
  };
  services: { llama: boolean; webui: boolean };
}

export interface InstallRequest {
  modelId: string;
  quant: string;
  context: number;
  gpuLayers: number;
}

export const status = () => call<AgentStatus>('GET', '/v1/status');
export const install = (req: InstallRequest) => call<{ started: boolean }>('POST', '/v1/install', req);
export const stop = () => call<{ stopped: boolean }>('POST', '/v1/stop');
