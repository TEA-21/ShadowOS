/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        brand: {
          50: '#F5F3FF',
          100: '#EDE9FE',
          200: '#DDD6FE',
          300: '#C4B5FD',
          400: '#8E67E8',
          500: '#7C3AED',
          600: '#6B46C1', // Primary Accent
          700: '#5B21B6',
          800: '#4C1D95',
          900: '#371B78',
        },
        status: {
          active: '#2E8B57', // Muted green for success/active states
          activeBg: '#F0FDF4',
          activeBorder: '#BBF7D0',
          warning: '#D97757', // Soft orange for warnings/notes
          warningBg: '#FFF7ED',
          warningBorder: '#FED7AA',
          error: '#DC2626',
          errorBg: '#FEF2F2',
          errorBorder: '#FECACA',
          stopped: '#64748B',
          stoppedBg: '#F1F5F9',
        },
        panel: {
          bg: '#FFFFFF',
          sidebar: '#FFFFFF',
          border: '#E5E7EB',
          subtle: '#F8FAFC',
        }
      },
      fontFamily: {
        sans: ['Inter', '-apple-system', 'BlinkMacSystemFont', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['"JetBrains Mono"', 'Fira Code', 'ui-monospace', 'SFMono-Regular', 'Menlo', 'Monaco', 'Consolas', 'monospace'],
      },
      boxShadow: {
        'subtle': '0 1px 3px 0 rgba(0, 0, 0, 0.05), 0 1px 2px 0 rgba(0, 0, 0, 0.03)',
        'card': '0 1px 3px 0 rgba(15, 23, 42, 0.06), 0 1px 2px -1px rgba(15, 23, 42, 0.04)',
        'panel': '0 4px 6px -1px rgba(15, 23, 42, 0.05), 0 2px 4px -2px rgba(15, 23, 42, 0.03)',
      }
    },
  },
  plugins: [],
}
