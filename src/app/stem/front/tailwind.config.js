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
      }
    },
  },
  plugins: [require('tailwindcss-safe-area')],
}
