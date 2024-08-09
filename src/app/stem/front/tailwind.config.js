/** @type {import('tailwindcss').Config} */
module.exports = {
  content: ["./**/*.rs"],
  theme: {
    extend: {
      colors: {
          // primary: '#E16462', // pale beige/pink
          // primary: '#B12A90', // purplish
          // primary: '#0ea5e9', // sky-500
          primary: '#0a84ff',
      },
      screens: {
          // Use "tall:[class]" to target taller screens. For iPhones models,
          // 700px is a threshold between square-screen models like the SE
          // (viewport height of 667px) and rounded-screen models, so we use it
          // to add buffer for the rounded edges.
          'tall': { 'raw': '(min-height: 700px)' },
      },
      keyframes: {
        appear: {
          '0%': { opacity: 0, transform: 'translate(-50%, -50%) scale(1.1)' },
        },
        appear2: {
          '0%': { opacity: 0, transform: 'translate(-0%, -50%)' },
        },
        inout: {
          '0%,100%': { opacity: 0 },
          '40%,60%': { opacity: 1 },
        },
      },
      animation: {
        appear: 'appear 0.1s ease-in-out',
        appear2: 'appear2 0.1s ease-in-out',
        inout: 'inout 1s ease-in-out',
      }
    },
  },
  plugins: [require('tailwindcss-safe-area')],
}
