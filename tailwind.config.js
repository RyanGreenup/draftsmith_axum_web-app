/** @type {import('tailwindcss').Config} */
module.exports = {
  content: [
    "./*.{html,js}",
    "./templates/**/*.{html,js}",
    "./static/css/{html,js}",
    "./static/styles/{html,js}",
    "./static/js/{html,js}",
  ],
  theme: {
    extend: {
      daisyui: {
        "--rounded-box": "1rem",
        "--rounded-btn": "0.5rem",
        "--rounded-badge": "1.9rem",
        "--animation-btn": "0.25s",
        "--animation-input": "0.2s",
        "--btn-focus-scale": "0.95",
        "--border-btn": "1px",
        "--tab-border": "1px",
        "--tab-radius": "0.5rem",
      },
    },
  },
  plugins: [require("daisyui")],
  daisyui: {
    themes: [
      "light",
      "dark",
      "cupcake",
      "bumblebee",
      "emerald",
      "corporate",
      "synthwave",
      "retro",
      "cyberpunk",
      "valentine",
      "halloween",
      "garden",
      "forest",
      "aqua",
      "lofi",
      "pastel",
      "fantasy",
      "wireframe",
      "black",
      "luxury",
      "dracula",
      "cmyk",
      "autumn",
      "business",
      "acid",
      "lemonade",
      "night",
      "coffee",
      "winter",
      "dim",
      "nord",
      "sunset",
      {
        catppucin: {
          "--rounded-box": "0rem",
          "--rounded-btn": "0rem",
          "--rounded-badge": "0rem",
          "--animation-btn": "0s",
          "--animation-input": "0s",
          "--btn-focus-scale": "0",
          "--border-btn": "0px",
          "--tab-border": "0px",
          "--tab-radius": "0rem",

          primary: "#c792ea",
          secondary: "#89b4fa",
          accent: "#91d7ff",
          neutral: "#1e1e2e",
          "base-100": "#2b213a",
          info: "#74c7ec",
          success: "#a6e3a1",
          warning: "#f9e2af",
          error: "#f38ba8",
        },
        aurora: {
          primary: "#d800ff",
          secondary: "#002cff",
          accent: "#00cdff",
          neutral: "#09140f",
          "base-100": "#2e2328",
          info: "#00beff",
          success: "#669e00",
          warning: "#d97100",
          error: "#cf002b",
        },
        twilight: {
          "--rounded-box": "0rem",
          "--rounded-btn": "0rem",
          "--rounded-badge": "0rem",
          "--animation-btn": "0s",
          "--animation-input": "0s",
          "--btn-focus-scale": "0",
          "--border-btn": "0px",
          "--tab-border": "0px",
          "--tab-radius": "0rem",

          primary: "#e100ff",
          secondary: "#00bd62",
          accent: "#654f00",
          neutral: "#1a1c26",
          "base-100": "#fff5ff",
          info: "#00c4ff",
          success: "#569a00",
          warning: "#da0000",
          error: "#ff7c84",
        },
      },
    ],
  },
};
