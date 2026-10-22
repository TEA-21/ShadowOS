import type { MicroVM, Snapshot, FileDiffItem, SystemInfo } from '../types';

const API_BASE = '/api';

export class ApiError extends Error {
  status: number;

  constructor(status: number, message: string) {
    super(message);
    this.name = 'ApiError';
    this.status = status;
  }
}

async function request<T>(endpoint: string, options?: RequestInit): Promise<T> {
  const url = `${API_BASE}${endpoint}`;
  try {
    const res = await fetch(url, {
      headers: {
        'Content-Type': 'application/json',
        ...options?.headers,
      },
      ...options,
    });

    if (!res.ok) {
      const errorText = await res.text().catch(() => 'Unknown error');
      throw new ApiError(res.status, `Request to ${endpoint} failed (${res.status}): ${errorText}`);
    }

    return await res.json() as T;
  } catch (err: unknown) {
    if (err instanceof ApiError) throw err;
    const msg = err instanceof Error ? err.message : String(err);
    throw new ApiError(0, `Network failure connecting to ${url}: ${msg}`);
  }
}

/** Check if ShadowOS backend is reachable and responsive */
export async function checkBackendHealth(): Promise<boolean> {
  try {
    const res = await fetch(`${API_BASE}/health`, { signal: AbortSignal.timeout(2000) });
    return res.ok;
  } catch {
    return false;
  }
}

/** Fetch host system and hypervisor runtime details */
export async function fetchSystemInfo(): Promise<SystemInfo> {
  return request<SystemInfo>('/system/info');
}

/** Fetch all microVMs */
export async function fetchVms(): Promise<MicroVM[]> {
  return request<MicroVM[]>('/vms');
}

/** Provision a new microVM */
export async function spawnVm(params: {
  name: string;
  hypervisor?: string;
  vcpus?: number;
  memoryMb?: number;
  workloadProfile?: string;
}): Promise<MicroVM> {
  return request<MicroVM>('/vms', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

/** Suspend microVM execution and trigger memory ballooning */
export async function pauseVm(id: string): Promise<MicroVM> {
  return request<MicroVM>(`/vms/${encodeURIComponent(id)}/pause`, {
    method: 'POST',
  });
}

/** Resume microVM execution */
export async function resumeVm(id: string): Promise<MicroVM> {
  return request<MicroVM>(`/vms/${encodeURIComponent(id)}/resume`, {
    method: 'POST',
  });
}

/** Safely terminate microVM and wipe ephemeral memory */
export async function killVm(id: string): Promise<{ status: string; id: string }> {
  return request<{ status: string; id: string }>(`/vms/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}

/** Fetch all differential memory and state snapshots */
export async function fetchSnapshots(): Promise<Snapshot[]> {
  return request<Snapshot[]>('/snapshots');
}

/** Create a new differential memory snapshot into /dev/shm */
export async function createSnapshot(params: {
  vmId: string;
  name: string;
  description?: string;
}): Promise<Snapshot> {
  return request<Snapshot>('/snapshots', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

/** Instant sub-100ms rollback restoring memory and wiping upperdir */
export async function rollbackSnapshot(id: string): Promise<{
  success: boolean;
  snapshotId: string;
  vmId: string;
  latencyMs: number;
}> {
  return request<{
    success: boolean;
    snapshotId: string;
    vmId: string;
    latencyMs: number;
  }>(`/snapshots/${encodeURIComponent(id)}/rollback`, {
    method: 'POST',
  });
}

/** Fetch file diff items for a microVM */
export async function fetchVmDiff(vmId: string): Promise<FileDiffItem[]> {
  return request<FileDiffItem[]>(`/vms/${encodeURIComponent(vmId)}/diff`);
}

/** Promote ephemeral upperdir changes to host workspace */
export async function promoteHostTree(params: {
  noteId?: string;
  vmId?: string;
  targetPath?: string;
}): Promise<{
  success: boolean;
  noteId: string;
  vmId: string;
  filesMerged: number;
  hostTreeChecksum: string;
}> {
  return request('/mcp/promote', {
    method: 'POST',
    body: JSON.stringify(params),
  });
}

/** Fetch aggregate swarm metrics */
export async function fetchSwarmSummary(): Promise<{
  activeWorkers: number;
  totalTasksCompleted: number;
  aggregatePeakMemoryMb: number;
  withinPeakMemoryLimit: boolean;
  zeroCrossPollution: boolean;
  workerReports: Array<{
    workerId: string;
    pinnedCpu: number;
    tasksExecuted: number;
    filesModified: number;
    modifiedPaths: string[];
    currentMemoryMb: number;
    status: string;
  }>;
}> {
  return request('/swarm/summary');
}
