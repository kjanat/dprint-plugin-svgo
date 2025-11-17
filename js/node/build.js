import * as esbuild from 'esbuild';
import { NodeModulesPolyfillPlugin } from '@esbuild-plugins/node-modules-polyfill';
import { NodeGlobalsPolyfillPlugin } from '@esbuild-plugins/node-globals-polyfill';

await esbuild.build({
  entryPoints: ['main.ts'],
  bundle: true,
  format: 'iife',
  outfile: 'dist/main.js',
  platform: 'browser',
  target: 'chrome58',
  plugins: [
    NodeModulesPolyfillPlugin(),
    NodeGlobalsPolyfillPlugin({
      buffer: true,
      process: true,
    }),
  ],
  define: {
    'process.env.NODE_ENV': '"production"',
  },
});

console.log('Build complete');
