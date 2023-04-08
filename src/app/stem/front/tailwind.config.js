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
      }
    },
  },
  plugins: [],
}
