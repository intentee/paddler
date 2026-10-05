import * as esbuild from "esbuild";
import { emptyDir } from "fs-extra";
import { glob } from "glob";
import { writeFile } from "node:fs/promises";

import { basic } from "jarmuz/job-types";

const metafileFilename = "esbuild-meta.json";
const outdir = "static";
const publicPath = "/static/";

export function jobEsbuild({ development }) {
  basic(async function ({ buildId, printSubtreeList, resetConsole }) {
    await resetConsole();
    await emptyDir(outdir);

    console.log(`Building with ID: ${buildId}`);

    const inject = await glob(["resources/ts/polyfill_*.{ts,tsx}"]);

    const entryPoints = await glob([
      "resources/css/{fragment,global,page}-*.css",
      "resources/css/reset.css",
      "resources/ts/{controller,global,worker}_*.{ts,tsx}",
    ]);

    printSubtreeList({
      title: "Entry points",
      items: entryPoints,
    });

    const settings = {
      outdir,
      bundle: true,
      entryPoints,
      minify: !development,
      sourcemap: true,
      splitting: true,
      format: "esm",
      target: "es2024",
      loader: {
        ".jpg": "file",
        ".otf": "file",
        ".png": "file",
        ".svg": "file",
        ".ttf": "file",
        ".webp": "file",
        ".woff2": "file",
      },
      assetNames: `[name]_${buildId}`,
      entryNames: `[name]_${buildId}`,
      metafile: true,
      define: {
        "process.env.NODE_ENV": JSON.stringify(
          development ? "development" : "production",
        ),
        __BUILD_ID: JSON.stringify(buildId),
        __DEV__: JSON.stringify(String(development)),
        __PUBLIC_PATH: JSON.stringify(publicPath),
      },
      inject,
      publicPath,
      preserveSymlinks: true,
      treeShaking: true,
      tsconfig: "tsconfig.json",
    };

    console.log("");

    const result = await esbuild.build(settings);

    await writeFile(metafileFilename, JSON.stringify(result.metafile));

    console.log(`Build metafile written to: ${metafileFilename}`);
    console.log(`Build finished with ID: ${buildId}`);

    if (result.errors.length > 0 || result.warnings.length > 0) {
      return false;
    }
  });
}
