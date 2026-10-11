import React from 'react';

export interface CardProps {
  children: React.ReactNode;
  title?: React.ReactNode;
  subtitle?: string;
  badge?: React.ReactNode;
  actions?: React.ReactNode;
  className?: string;
  headerClassName?: string;
  bodyClassName?: string;
  padding?: 'none' | 'sm' | 'md' | 'lg';
}

export const Card: React.FC<CardProps> = ({
  children,
  title,
  subtitle,
  badge,
  actions,
  className = '',
  headerClassName = '',
  bodyClassName = '',
  padding = 'md',
}) => {
  const paddingClasses = {
    none: 'p-0',
    sm: 'p-3',
    md: 'p-4 sm:p-5',
    lg: 'p-6',
  };

  const hasHeader = title || subtitle || actions || badge;

  return (
    <div className={`bg-white border border-gray-200 rounded-lg shadow-subtle overflow-hidden ${className}`}>
      {hasHeader && (
        <div className={`px-4 py-3 sm:px-5 sm:py-3.5 border-b border-gray-200 flex flex-wrap items-center justify-between gap-3 bg-white ${headerClassName}`}>
          <div className="flex items-center gap-2.5 min-w-0">
            {title && typeof title === 'string' ? (
              <h3 className="text-sm font-semibold text-slate-800 tracking-tight truncate">{title}</h3>
            ) : (
              title
            )}
            {badge}
            {subtitle && (
              <span className="text-xs text-slate-500 font-normal hidden sm:inline">{subtitle}</span>
            )}
          </div>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </div>
      )}
      <div className={`${paddingClasses[padding]} ${bodyClassName}`}>
        {children}
      </div>
    </div>
  );
};
