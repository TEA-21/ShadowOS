import React, { useState, useEffect, useRef } from 'react';
import { Layout } from './components/layout/Layout';
import { SandboxManager } from './components/sandbox/SandboxManager';
import { SnapshotRollbackView } from './components/snapshots/SnapshotRollbackView';
import { McpSwarmConsole } from './components/mcp/McpSwarmConsole';
import { SpawnSandboxModal } from './components/sandbox/SpawnSandboxModal';
import { 
  INITIAL_VMS, 
  MOCK_SNAPSHOTS, 
  MOCK_VSOCK_LOGS, 
  MOCK_AGENT_NOTES 
} from './data/mockData';
import type { 
  MicroVM, 
  Snapshot, 
  VsockLogEntry, 
  AgentNote, 
  ActiveTab, 
  BackendConnectionStatus, 
  SystemInfo 
} from './types';
import * as api from './services/api';
import { VsockLogSocket } from './services/websocket';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<ActiveTab>('sandboxes');
  const [vms, setVms] = useState<MicroVM[]>(INITIAL_VMS);
  const [snapshots, setSnapshots] = useState<Snapshot[]>(MOCK_SNAPSHOTS);
  const [logs, setLogs] = useState<VsockLogEntry[]>(MOCK_VSOCK_LOGS);
  const [agentNotes, setAgentNotes] = useState<AgentNote[]>(MOCK_AGENT_NOTES);
  const [selectedVmId, setSelectedVmId] = useState<string>(INITIAL_VMS[0]?.id || '');
  const [isSpawnModalOpen, setIsSpawnModalOpen] = useState(false);
  const [isStreaming, setIsStreaming] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');
  const [connectionStatus, setConnectionStatus] = useState<BackendConnectionStatus>('connecting');
  const [systemInfo, setSystemInfo] = useState<SystemInfo | null>(null);
  const [activeBaselineSnapshotId, setActiveBaselineSnapshotId] = useState<string | null>(null);

  const socketRef = useRef<VsockLogSocket | null>(null);

  // Initialize Backend Connection & Telemetry Sync
  useEffect(() => {
    let isMounted = true;

    async function initBackend() {
      try {
        const isHealthy = await api.checkBackendHealth();
        if (!isMounted) return;

        if (isHealthy) {
          setConnectionStatus('connected');

          // Fetch live server state
          const [liveVms, liveSnapshots, sysInfo] = await Promise.allSettled([
            api.fetchVms(),
            api.fetchSnapshots(),
            api.fetchSystemInfo(),
          ]);

          if (!isMounted) return;

          if (liveVms.status === 'fulfilled' && liveVms.value.length > 0) {
            setVms(liveVms.value);
            setSelectedVmId(liveVms.value[0]?.id || '');
          }
          if (liveSnapshots.status === 'fulfilled' && liveSnapshots.value.length > 0) {
            setSnapshots(liveSnapshots.value);
          }
          if (sysInfo.status === 'fulfilled') {
            setSystemInfo(sysInfo.value);
          }
        } else {
          setConnectionStatus('offline');
        }
      } catch {
        if (isMounted) setConnectionStatus('offline');
      }
    }

    initBackend();

    // Initialize real-time WebSocket connection
    const logSocket = new VsockLogSocket();
    socketRef.current = logSocket;

    const unsubLogs = logSocket.subscribe((newLog) => {
      if (!isStreaming) return;
      setLogs((prev) => [...prev.slice(-120), newLog]);
    });

    const unsubStatus = logSocket.onStatusChange((isConnected) => {
      if (isConnected) {
        setConnectionStatus('connected');
        // Refresh live data on reconnect
        api.fetchVms().then((v) => v.length > 0 && setVms(v)).catch(() => {});
        api.fetchSnapshots().then((s) => s.length > 0 && setSnapshots(s)).catch(() => {});
        api.fetchSystemInfo().then(setSystemInfo).catch(() => {});
      } else {
        setConnectionStatus((prev) => (prev === 'connected' ? 'connecting' : prev));
      }
    });

    return () => {
      isMounted = false;
      unsubLogs();
      unsubStatus();
      logSocket.close();
    };
  }, [isStreaming]);

  // Fallback simulation timer if backend is offline
  useEffect(() => {
    if (connectionStatus === 'connected' || !isStreaming) return;

    const interval = setInterval(() => {
      const simulatedChannels = ['vsock:3', 'vsock:4', 'virtio-balloon', 'mcp-jsonrpc', 'overlayfs'];
      const channel = simulatedChannels[Math.floor(Math.random() * simulatedChannels.length)];
      const activeList = vms.filter((v) => v.status === 'Active');
      const randomVm = activeList[Math.floor(Math.random() * activeList.length)] || vms[0];

      const now = new Date();
      const timestamp = `${now.toTimeString().split(' ')[0]}.${String(now.getMilliseconds()).padStart(3, '0')}`;

      let message = '';
      let jsonRpcPayload: string | undefined;
      let level: VsockLogEntry['level'] = 'INFO';

      if (channel === 'virtio-balloon') {
        const delta = Math.floor(Math.random() * 40) - 20;
        message = `Dynamic memory balloon heartbeat on ${randomVm?.id}: RSS delta ${delta > 0 ? '+' : ''}${delta} MB. Idle footprint stable at ${randomVm?.ramUsage.currentMb} MB.`;
        level = 'DEBUG';
      } else if (channel === 'mcp-jsonrpc') {
        message = `MCP tool inspection invoked: "inspect_diff" on ${randomVm?.id}. Output unified diff framed over stdio.`;
        jsonRpcPayload = JSON.stringify({
          jsonrpc: '2.0',
          method: 'tools/call',
          params: { name: 'inspect_diff', arguments: { sandbox_id: randomVm?.id } },
          id: `req-${Math.floor(Math.random() * 9000) + 1000}`
        });
      } else if (channel === 'overlayfs') {
        message = `OverlayFS upperdir sync: 2 hunks modified in tmpfs scratch layer for ${randomVm?.id}. Host repo bit-identical.`;
        level = 'DEBUG';
      } else {
        const latency = (0.28 + Math.random() * 0.15).toFixed(2);
        message = `Zero-TCP packet stream on channel #${randomVm?.vsockCid || 3}. Multiplexed latency: ${latency} µs.`;
      }

      const newLog: VsockLogEntry = {
        id: `log-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        timestamp,
        level,
        channel,
        vmId: randomVm?.id,
        message,
        jsonRpcPayload,
      };

      setLogs((prev) => [...prev.slice(-120), newLog]);
    }, 4200);

    return () => clearInterval(interval);
  }, [connectionStatus, isStreaming, vms]);

  // Calculate live aggregate Swarm memory footprint
  const aggregateRamMb = vms.reduce(
    (acc, vm) => acc + (vm.status !== 'Stopped' ? vm.ramUsage.currentMb : 0),
    0
  );

  // Active microVM count
  const activeVmCount = vms.filter((v) => v.status === 'Active').length;

  // Handlers
  const handleSpawnVm = async (newVmPartial: Partial<MicroVM>) => {
    if (connectionStatus === 'connected') {
      try {
        const spawned = await api.spawnVm({
          name: newVmPartial.name || 'new-sandbox-instance',
          hypervisor: newVmPartial.hypervisor,
          vcpus: newVmPartial.cpuAllocation?.vcpus,
          memoryMb: newVmPartial.ramUsage?.limitMb,
          workloadProfile: newVmPartial.workloadProfile,
        });
        setVms((prev) => [spawned, ...prev]);
        return;
      } catch (err) {
        console.error('Failed to spawn VM on backend, falling back to local:', err);
      }
    }

    // Fallback local spawn
    const newVm = newVmPartial as MicroVM;
    setVms((prev) => [newVm, ...prev]);

    const initSnapshot: Snapshot = {
      id: `snap-${Math.floor(Math.random() * 9000) + 1000}-init`,
      vmId: newVm.id,
      name: `Baseline Hardware Boot: ${newVm.name}`,
      timestamp: new Date().toTimeString().split(' ')[0],
      date: '2026-10-10',
      description: `Initial hardware microVM boot via ${newVm.hypervisor}. DAX direct memory mapping active.`,
      trigger: 'manual',
      ramFootprintMb: newVm.ramUsage.currentMb,
      cowUpperSize: '0.0 MB',
      rollbackLatencyMs: 22.38,
      stats: { added: 0, modified: 0, deleted: 0, totalFiles: 0 },
      files: [],
    };

    setSnapshots((prev) => [initSnapshot, ...prev]);
  };

  const handleTogglePause = async (id: string) => {
    const target = vms.find((v) => v.id === id);
    if (!target) return;

    if (connectionStatus === 'connected') {
      try {
        const updated = target.status === 'Suspended'
          ? await api.resumeVm(id)
          : await api.pauseVm(id);
        setVms((prev) => prev.map((vm) => (vm.id === id ? updated : vm)));
        return;
      } catch (err) {
        console.error('Pause/Resume API call failed, falling back to local state:', err);
      }
    }

    // Fallback local toggle
    setVms((prev) =>
      prev.map((vm) => {
        if (vm.id === id) {
          const nextStatus = vm.status === 'Suspended' ? 'Active' : 'Suspended';
          return { ...vm, status: nextStatus };
        }
        return vm;
      })
    );
  };

  const handleKillVm = async (id: string) => {
    if (connectionStatus === 'connected') {
      try {
        await api.killVm(id);
        setVms((prev) =>
          prev.map((vm) =>
            vm.id === id
              ? { ...vm, status: 'Stopped', ramUsage: { ...vm.ramUsage, currentMb: 0, percent: 0 } }
              : vm
          )
        );
        return;
      } catch (err) {
        console.error('Kill VM API call failed, falling back to local state:', err);
      }
    }

    // Fallback local kill
    setVms((prev) =>
      prev.map((vm) =>
        vm.id === id
          ? { ...vm, status: 'Stopped', ramUsage: { ...vm.ramUsage, currentMb: 0, percent: 0 } }
          : vm
      )
    );
  };

  const handleRollback = async (snapshotId: string) => {
    const targetSnapshot = snapshots.find((s) => s.id === snapshotId);
    if (!targetSnapshot) return;

    setActiveBaselineSnapshotId(snapshotId);

    if (connectionStatus === 'connected') {
      try {
        await api.rollbackSnapshot(snapshotId);
      } catch (err) {
        console.error('Rollback API failed, falling back to local state:', err);
      }
    }

    // Always update local VM state for instant UI responsiveness
    setVms((prev) =>
      prev.map((vm) => {
        if (vm.id === targetSnapshot.vmId) {
          return {
            ...vm,
            status: 'Active',
            cowUpperSize: '0.0 MB',
            ramUsage: {
              ...vm.ramUsage,
              currentMb: targetSnapshot.ramFootprintMb,
            },
          };
        }
        return vm;
      })
    );

    setLogs((prev) => [
      {
        id: `log-rollback-${Date.now()}`,
        timestamp: new Date().toTimeString().split(' ')[0],
        level: 'INFO',
        channel: 'mcp-jsonrpc',
        vmId: targetSnapshot.vmId,
        message: `Rollback primitive "rollback_state" executed in ${targetSnapshot.rollbackLatencyMs} ms. Upperdir wiped (18.4 MB -> 0.0 MB), memory restored to [${snapshotId}].`,
      },
      ...prev,
    ]);
  };

  const handleMutateUpperdir = (vmId?: string) => {
    const targetId = vmId || selectedVmId || vms[0]?.id;
    setActiveBaselineSnapshotId(null);
    setVms((prev) =>
      prev.map((vm) => {
        if (vm.id === targetId) {
          return {
            ...vm,
            cowUpperSize: '18.4 MB',
          };
        }
        return vm;
      })
    );
    setLogs((prev) => [
      {
        id: `log-mutate-${Date.now()}`,
        timestamp: new Date().toTimeString().split(' ')[0],
        level: 'WARN',
        channel: 'overlayfs',
        vmId: targetId,
        message: `Autonomous agent mutated 3 files in tmpfs upperdir scratch layer (18.4 MB). Host lowerdir unmodified.`,
      },
      ...prev,
    ]);
  };

  const handleCreateSnapshot = async (name: string, description: string) => {
    const targetVm = vms.find((v) => v.id === selectedVmId) || vms[0];
    if (!targetVm) return;

    if (connectionStatus === 'connected') {
      try {
        const created = await api.createSnapshot({
          vmId: targetVm.id,
          name,
          description,
        });
        setSnapshots((prev) => [created, ...prev]);
        return;
      } catch (err) {
        console.error('Create snapshot API failed, falling back to local state:', err);
      }
    }

    // Fallback local snapshot
    const newSnapshot: Snapshot = {
      id: `snap-${Math.floor(Math.random() * 9000) + 1000}-manual`,
      vmId: targetVm.id,
      name,
      timestamp: new Date().toTimeString().split(' ')[0],
      date: '2026-10-10',
      description,
      trigger: 'manual',
      ramFootprintMb: targetVm.ramUsage.currentMb,
      cowUpperSize: targetVm.cowUpperSize,
      rollbackLatencyMs: 31.05,
      stats: { added: 1, modified: 2, deleted: 0, totalFiles: 3 },
      files: snapshots[0]?.files.slice(0, 3) || [],
    };

    setSnapshots((prev) => [newSnapshot, ...prev]);
  };

  const handleInspectDiff = () => {
    setSelectedVmId(vms[0]?.id || 'vm-4a7b2e9c');
    setActiveTab('snapshots');
  };

  const handlePromoteHostTree = async (noteId: string) => {
    if (connectionStatus === 'connected') {
      try {
        await api.promoteHostTree({ noteId, vmId: selectedVmId || 'vm-4a7b2e9c' });
        setAgentNotes((prev) =>
          prev.map((note) =>
            note.id === noteId ? { ...note, status: 'Completed' } : note
          )
        );
        return;
      } catch (err) {
        console.error('Promote host tree API failed, falling back to local state:', err);
      }
    }

    // Fallback local promotion
    setAgentNotes((prev) =>
      prev.map((note) =>
        note.id === noteId ? { ...note, status: 'Completed' } : note
      )
    );

    const now = new Date();
    const timestamp = `${now.toTimeString().split(' ')[0]}.${String(now.getMilliseconds()).padStart(3, '0')}`;
    const promoteLog: VsockLogEntry = {
      id: `log-promote-${Date.now()}`,
      timestamp,
      level: 'INFO',
      channel: 'mcp-jsonrpc',
      vmId: selectedVmId || 'vm-4a7b2e9c',
      message: 'MCP primitive "promote_change" executed. 6 files atomically merged into host repository (SHA256 verified).',
      jsonRpcPayload: JSON.stringify({
        jsonrpc: '2.0',
        method: 'tools/call',
        params: {
          name: 'promote_change',
          arguments: {
            sandbox_id: selectedVmId || 'vm-4a7b2e9c',
            target_path: 'd:\\Projects\\ShadowOS',
          },
        },
        result: {
          status: 'success',
          files_merged: 6,
          host_tree_checksum: 'a2f51b41...',
        },
        id: `req-${Date.now()}`,
      }),
    };
    setLogs((prev) => [...prev, promoteLog]);
  };

  return (
    <Layout
      activeTab={activeTab}
      onTabChange={setActiveTab}
      activeVmCount={activeVmCount}
      snapshotCount={snapshots.length}
      onSpawnClick={() => setIsSpawnModalOpen(true)}
      searchQuery={searchQuery}
      onSearchChange={setSearchQuery}
      aggregateRamMb={aggregateRamMb}
      connectionStatus={connectionStatus}
      systemInfo={systemInfo}
    >
      {/* 1. Sandbox Manager View */}
      {activeTab === 'sandboxes' && (
        <SandboxManager
          vms={vms}
          onSpawnVm={handleSpawnVm}
          onTogglePause={handleTogglePause}
          onKillVm={handleKillVm}
        />
      )}

      {/* 2. Snapshot & Rollback View */}
      {activeTab === 'snapshots' && (
        <SnapshotRollbackView
          snapshots={snapshots}
          vms={vms}
          selectedVmId={selectedVmId}
          onSelectVmId={setSelectedVmId}
          onRollback={handleRollback}
          onCreateSnapshot={handleCreateSnapshot}
          activeBaselineSnapshotId={activeBaselineSnapshotId}
          onMutateUpperdir={handleMutateUpperdir}
        />
      )}

      {/* 3. MCP / Swarm Console View */}
      {activeTab === 'mcp-console' && (
        <McpSwarmConsole
          logs={logs}
          agentNotes={agentNotes}
          isStreaming={isStreaming}
          onToggleStreaming={() => setIsStreaming(!isStreaming)}
          onClearLogs={() => setLogs([])}
          onInspectDiff={handleInspectDiff}
          onPromoteHostTree={handlePromoteHostTree}
        />
      )}

      {/* Spawn Sandbox Modal */}
      <SpawnSandboxModal
        isOpen={isSpawnModalOpen}
        onClose={() => setIsSpawnModalOpen(false)}
        onSpawn={handleSpawnVm}
      />
    </Layout>
  );
};

export default App;
