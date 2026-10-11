import React from 'react';

export interface BadgeProps {
  children: React.ReactNode;
  variant?: 'active' | 'warning' | 'error' | 'stopped' | 'purple' | 'neutral' | 'mono';
  size?: 'sm' | 'md';
  className?: string;
  dot?: boolean;
}

export const Badge: React.FC<BadgeProps> = ({
  children,
  variant = 'neutral',
  size = 'md',
  className = '',
  dot = false,
}) => {
  const baseClasses = 'inline-flex items-center font-medium rounded-md border transition-colors';
  
  const sizeClasses = size === 'sm' 
    ? 'px-1.5 py-0.5 text-xs' 
    : 'px-2 py-0.5 text-xs';

  const variantClasses = {
    // Muted green for success/active states (#2E8B57)
    active: 'bg-emerald-50 text-[#2E8B57] border-emerald-200/80',
    // Soft orange for warnings/notes (#D97757)
    warning: 'bg-amber-50 text-[#D97757] border-amber-200/80',
    // Soft red for error/deleted
    error: 'bg-rose-50 text-rose-700 border-rose-200/80',
    // Neutral stopped state
    stopped: 'bg-slate-100 text-slate-600 border-slate-200',
    // Primary Purple (#6B46C1)
    purple: 'bg-purple-50 text-purple-700 border-purple-200',
    // Standard subtle neutral
    neutral: 'bg-slate-50 text-slate-700 border-slate-200',
    // Monospace code tag
    mono: 'bg-slate-100 text-slate-800 border-slate-200 font-mono text-[11px]',
  };

  const dotClasses = {
    active: 'bg-[#2E8B57]',
    warning: 'bg-[#D97757]',
    error: 'bg-rose-600',
    stopped: 'bg-slate-400',
    purple: 'bg-purple-600',
    neutral: 'bg-slate-400',
    mono: 'bg-slate-500',
  };

  return (
    <span className={`${baseClasses} ${sizeClasses} ${variantClasses[variant]} ${className}`}>
      {dot && (
        <span className={`w-1.5 h-1.5 rounded-full mr-1.5 shrink-0 ${dotClasses[variant]}`} />
      )}
      {children}
    </span>
  );
};
