import type { Config } from "tailwindcss";

// System Trace - "Signal" palette, encoded as Tailwind tokens.
//
// Per DESIGN_SPEC.md section 9, every Signal hex from BRAND.md is declared as a
// CSS custom property on :root (light) and .dark (dark) in src/theme/theme.css.
// Here we map the token NAMES to those CSS vars so utilities like
// `bg-surface text-text-muted border-border` resolve per theme, and uPlot charts
// can read the same variables. The raw hex values are kept in comments so this
// file stays a readable record of the palette.
//
// Dark / Light hex reference (BRAND.md):
//   bg          #0D1117 / #FFFFFF
//   surface     #161B22 / #F6F8FA
//   surface-2   #1C2128 / #FFFFFF   (derived raised step, DESIGN_SPEC 1.1)
//   border      #21262D / #D0D7DE
//   text        #E6EDF3 / #1F2328
//   text-muted  #8B949E / #656D76
//   accent      #2DD4BF / #0EA5A0
//   positive    #34D399 / #2EA043
//   warning     #F59E0B / #D29922
//   negative    #F87171 / #E5534B

const config: Config = {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        bg: "var(--bg)",
        surface: "var(--surface)",
        "surface-2": "var(--surface-2)",
        border: "var(--border)",
        text: "var(--text)",
        "text-muted": "var(--text-muted)",
        accent: "var(--accent)",
        positive: "var(--positive)",
        warning: "var(--warning)",
        negative: "var(--negative)",
      },
      fontFamily: {
        sans: [
          "Inter",
          "-apple-system",
          '"Segoe UI"',
          "Roboto",
          "system-ui",
          "sans-serif",
        ],
        mono: [
          "ui-monospace",
          '"SF Mono"',
          '"Cascadia Code"',
          "Consolas",
          "monospace",
        ],
      },
      // Type scale from DESIGN_SPEC.md section 2 (size / line-height).
      fontSize: {
        display: ["56px", { lineHeight: "60px", letterSpacing: "-0.02em", fontWeight: "700" }],
        h1: ["28px", { lineHeight: "34px", letterSpacing: "-0.01em", fontWeight: "600" }],
        h2: ["20px", { lineHeight: "28px", letterSpacing: "-0.01em", fontWeight: "600" }],
        h3: ["16px", { lineHeight: "24px", fontWeight: "600" }],
        stat: ["24px", { lineHeight: "30px", letterSpacing: "-0.01em", fontWeight: "600" }],
        body: ["14px", { lineHeight: "22px", fontWeight: "400" }],
        "body-strong": ["14px", { lineHeight: "22px", fontWeight: "500" }],
        label: ["12px", { lineHeight: "16px", letterSpacing: "0.01em", fontWeight: "500" }],
        "mono-num": ["13px", { lineHeight: "20px", fontWeight: "500" }],
      },
      // 4px spacing scale (DESIGN_SPEC.md section 4). Tailwind's default scale
      // already covers these multiples; named aliases are added for the tokens
      // the spec references explicitly.
      borderRadius: {
        sm: "8px",
        md: "12px",
        lg: "16px",
        full: "9999px",
      },
      boxShadow: {
        // e1 cards
        e1: "0 1px 2px rgba(0,0,0,.06)",
        "e1-dark": "0 1px 2px rgba(0,0,0,.4)",
        // e2 popovers / dropdowns / toasts
        e2: "0 8px 24px rgba(0,0,0,.12)",
        "e2-dark": "0 8px 24px rgba(0,0,0,.5)",
        // e3 modals / onboarding
        e3: "0 16px 48px rgba(0,0,0,.18)",
        "e3-dark": "0 16px 48px rgba(0,0,0,.6)",
      },
      transitionDuration: {
        // Quiet motion defaults (DESIGN_SPEC.md section 1).
        hover: "150ms",
        theme: "150ms",
      },
    },
  },
  plugins: [],
};

export default config;
