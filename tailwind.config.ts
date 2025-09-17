import type { Config } from 'tailwindcss'
import animate from "tailwindcss-animate";

export default {
  content: [
    './index.html',
    './src/**/*.{ts,tsx}',
    "./pages/**/*.{ts,tsx}",
    "./components/**/*.{ts,tsx}",
    "./app/**/*.{ts,tsx}"
  ],
  plugins: [animate],
} satisfies Config
