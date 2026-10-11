import React from 'react';
import { 
  Layers, 
  History, 
  Terminal, 
  ShieldCheck
} from 'lucide-react';
import type { ActiveTab } from '../../types';

interface SidebarProps {
  activeTab: ActiveTab;
  onTabChange: (tab: ActiveTab) => void;
  activeVmCount: number;
  snapshotCount: number;
}

export const Sidebar: React.FC<SidebarProps> = ({
  activeTab,
  onTabChange,
  activeVmCount,
  snapshotCount,
}) => {
  const navItems = [
    {
      id: 'sandboxes' as ActiveTab,
      label: 'Sandbox Manager',
      description: 'Active microVM execution pool',
      icon: <Layers className="w-4 h-4" />,
      badge: `${activeVmCount} Active`,
      badgeVariant: 'active' as const,
    },
    {
      id: 'snapshots' as ActiveTab,
      label: 'Snapshot & Rollback',
      description: 'CoW OverlayFS & git diffs',
      icon: <History className="w-4 h-4" />,
      badge: `${snapshotCount} States`,
      badgeVariant: 'purple' as const,
    },
    {
      id: 'mcp-console' as ActiveTab,
      label: 'MCP / Swarm Console',
      description: 'vsock logs & agent orchestration',
      icon: <Terminal className="w-4 h-4" />,
      badge: 'Live',
      badgeVariant: 'warning' as const,
    },
  ];

  return (
    <aside className="w-64 bg-white border-r border-slate-200 flex flex-col shrink-0 h-screen sticky top-0 select-none z-20">
      {/* Brand Header */}
      <div className="h-16 px-5 border-b border-slate-200 flex items-center justify-between bg-white">
        <div className="flex items-center gap-2.5">
          <div className="w-8 h-8 rounded-lg bg-purple-600 flex items-center justify-center text-white shadow-sm ring-2 ring-purple-100">
            <ShieldCheck className="w-5 h-5" />
          </div>
          <div>
            <div className="flex items-center gap-1.5">
              <span className="font-bold text-slate-900 tracking-tight text-sm">ShadowOS</span>
              <span className="text-[10px] uppercase font-mono font-semibold px-1.5 py-0.2 rounded bg-purple-100 text-purple-700">v1.0</span>
            </div>
            <p className="text-[11px] text-slate-400 font-normal">MicroVM Platform</p>
          </div>
        </div>
      </div>

      {/* Navigation Section */}
      <div className="p-3 flex-1 overflow-y-auto space-y-4">
        <div>
          <div className="px-3 pb-2 text-[10px] font-semibold text-slate-400 uppercase tracking-wider">
            Platform Views
          </div>
          <nav className="space-y-1">
            {navItems.map((item) => {
              const isActive = activeTab === item.id;
              return (
                <button
                  key={item.id}
                  onClick={() => onTabChange(item.id)}
                  className={`w-full group text-left px-3 py-2.5 rounded-lg text-xs font-medium transition-all duration-150 flex items-center justify-between ${
                    isActive
                      ? 'bg-purple-50 text-purple-900 font-semibold border border-purple-200 shadow-xs'
                      : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50 border border-transparent'
                  }`}
                >
                  <div className="flex items-center gap-2.5 min-w-0">
                    <span
                      className={`shrink-0 transition-colors ${
                        isActive ? 'text-purple-600' : 'text-slate-400 group-hover:text-slate-600'
                      }`}
                    >
                      {item.icon}
                    </span>
                    <div className="min-w-0 text-left">
                      <div className="truncate">{item.label}</div>
                      <div className="text-[10px] text-slate-400 font-normal truncate">
                        {item.description}
                      </div>
                    </div>
                  </div>
                  <span
                    className={`text-[10px] font-mono px-1.5 py-0.5 rounded border shrink-0 ${
                      isActive
                        ? 'bg-purple-100 text-purple-700 border-purple-200'
                        : 'bg-slate-100 text-slate-500 border-slate-200'
                    }`}
                  >
                    {item.badge}
                  </span>
                </button>
              );
            })}
          </nav>
        </div>
      </div>

      {/* Minimal Footer */}
      <div className="p-3.5 border-t border-slate-200 bg-slate-50/60 flex items-center justify-between text-[11px] text-slate-500">
        <div className="flex items-center gap-1.5">
          <span className="w-2 h-2 rounded-full bg-[#2E8B57]" />
          <span className="font-mono text-[11px] text-slate-600">vsock CID: 3..7</span>
        </div>
        <span className="text-[10px] text-slate-400 font-mono">libkrun / FC</span>
      </div>
    </aside>
  );
};
