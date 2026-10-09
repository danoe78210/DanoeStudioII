// tailwind.config.cjs
module.exports = {
  content: ["./index.html", "./src/**/*.{js,ts,jsx,tsx}"],
  theme: {
    extend: {
      colors: {
        paper: "#f5f5f0",
        parchment: "#f8f3e6",
      },
      fontFamily: {
        serif: ["Georgia", "serif"],
        sans: ["Helvetica Neue", "Arial", "sans-serif"],
      },
      boxShadow: {
        book: "0 10px 30px -5px rgba(0, 0, 0, 0.1), 0 4px 6px -4px rgba(0, 0, 0, 0.05)",
      },
    },
  },
  plugins: [],
};