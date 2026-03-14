import { build } from "esbuild";
import { copyFileSync } from "fs";

const development = process.argv.includes("--dev");

await build({
  entryPoints: ["src/main.tsx"],
  outdir: "dist",
  bundle: true,
  minify: !development,
  sourcemap: true,
  format: "esm",
  target: "es2024",
  loader: {
    ".png": "file",
    ".svg": "file",
    ".woff2": "file",
    ".ttf": "file",
  },
  define: {
    "process.env.NODE_ENV": JSON.stringify(
      development ? "development" : "production"
    ),
  },
  tsconfig: "tsconfig.json",
});

copyFileSync("index.html", "dist/index.html");
