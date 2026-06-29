/** @type {import('tailwindcss').Config} */
export default {
  content: ['./index.html', './src/**/*.{js,ts,jsx,tsx}'],
  theme: {
    extend: {
      colors: {
        cyber: {
          bg: '#0a0a0f',
          card: '#12121a',
          border: '#1a1a2e',
          cyan: '#00f3ff',
          magenta: '#ff00e6',
          yellow: '#ffea00',
          green: '#00ff88',
          red: '#ff0044',
          text: '#e0e0e0',
          muted: '#8888aa'
        }
      },
      fontFamily: {
        orbitron: ['Orbitron', 'sans-serif'],
        mono: ['JetBrains Mono', 'monospace']
      },
      boxShadow: {
        glow: '0 0 20px rgba(0, 243, 255, 0.15)',
        'glow-lg': '0 0 40px rgba(0, 243, 255, 0.25)'
      },
      backgroundImage: {
        'scan-grid':
          'linear-gradient(rgba(0,243,255,0.05) 1px, transparent 1px), linear-gradient(90deg, rgba(255,0,230,0.035) 1px, transparent 1px)'
      }
    }
  },
  plugins: []
}
