export type VmStatus = 'Active' | 'Suspended' | 'Stopped';

export interface MicroVM {
  id: string;
  name: string;
  hypervisor: 'libkrun' | 'firecracker';
  status: VmStatus;
  uptime: string;
  ramUsage: {
    currentMb: number;
    limitMb: number;
    balloonedMb?: number;
    percent: number;
  };
  cpuAllocation: {
    vcpus: number;
    usagePercent: number;
    pinnedCore?: number;
  };
  cowUpperSize: string;
  vsockCid: number;
  guestAgentStatus: 'Connected' | 'Syncing' | 'Offline';
  workloadProfile: string;
  ipAddress: string;
  createdAt: string;
}

export type DiffFileStatus = 'added' | 'modified' | 'deleted';

export interface DiffLine {
  type: 'add' | 'delete' | 'context' | 'hunk';
  text: string;
  oldLineNumber?: number;
  newLineNumber?: number;
  highlightWords?: string[];
}

export interface FileDiffItem {
  path: string;
  status: DiffFileStatus;
  additions: number;
  deletions: number;
  layer: 'upperdir (tmpfs RW)' | 'merged (/workspace)';
  diffLines: DiffLine[];
  aiExplanation?: string[];
}

export interface Snapshot {
  id: string;
  vmId: string;
  name: string;
  timestamp: string;
  date: string;
  description: string;
  trigger: 'manual' | 'pre-command' | 'swarm-checkpoint' | 'milestone';
  ramFootprintMb: number;
  cowUpperSize: string;
  rollbackLatencyMs: number;
  stats: {
    added: number;
    modified: number;
    deleted: number;
    totalFiles: number;
  };
  files: FileDiffItem[];
  overallAiSummary?: string[];
}

export interface VsockLogEntry {
  id: string;
  timestamp: string;
  level: 'INFO' | 'TRACE' | 'DEBUG' | 'WARN' | 'ERROR';
  channel: string;
  vmId?: string;
  message: string;
  jsonRpcPayload?: string;
}

export interface AgentNote {
  id: string;
  agentId: string;
  title: string;
  targetTask: string;
  timestamp: string;
  status: 'In Progress' | 'Completed' | 'Review Required';
  metrics: {
    testPassed: number;
    testTotal: number;
    compilationTimeMs: number;
    uncommittedDiffBytes: string;
  };
  coordinator: string;
  hypervisorDriver: string;
  filesystemStatus: string;
  sandboxingPolicy: string;
  nfrAudits: {
    id: string;
    label: string;
    target: string;
    measured: string;
    status: 'PASS' | 'WARN';
  }[];
  keyObservations: string[];
  recommendedActions: string[];
}

export type ActiveTab = 'sandboxes' | 'snapshots' | 'mcp-console';
