/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./popup.html", "./src/**/*.{js,ts,jsx,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      fontFamily: {
        sans: [
          "Segoe UI Variable",
          "Segoe UI",
          "Microsoft YaHei UI",
          "PingFang SC",
          "system-ui",
          "sans-serif",
        ],
      },
      boxShadow: {
        card: "0 20px 50px -12px rgba(15, 23, 42, 0.25)",
        "card-dark": "0 24px 60px -12px rgba(0, 0, 0, 0.65)",
      },
      animation: {
        "fade-up": "fadeUp 0.22s cubic-bezier(0.16, 1, 0.3, 1) forwards",
      },
      keyframes: {
        fadeUp: {
          "0%": { opacity: "0", transform: "translateY(8px) scale(0.985)" },
          "100%": { opacity: "1", transform: "translateY(0) scale(1)" },
        },
      },
    },
  },
  plugins: [],
};
