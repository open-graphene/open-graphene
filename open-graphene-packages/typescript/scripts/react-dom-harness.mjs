import { JSDOM } from 'jsdom';
import { build } from 'esbuild';
import { createRequire } from 'node:module';
import { mkdtemp, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
const root = fileURLToPath(new URL('../', import.meta.url));
/** Run real React DOM against a simulated DOM; this does not replace the Chromium suite. */
export async function withReactDom(version, contents, run) {
  const dom = new JSDOM('<!doctype html><body></body>', {
    url: 'https://sdk.test/',
  });
  const previous = new Map();
  for (const name of [
    'window',
    'document',
    'navigator',
    'HTMLElement',
    'MutationObserver',
  ]) {
    previous.set(name, Object.getOwnPropertyDescriptor(globalThis, name));
    Object.defineProperty(globalThis, name, {
      configurable: true,
      value: dom.window[name],
    });
  }
  previous.set(
    'IS_REACT_ACT_ENVIRONMENT',
    Object.getOwnPropertyDescriptor(globalThis, 'IS_REACT_ACT_ENVIRONMENT'),
  );
  previous.set(
    'MessageChannel',
    Object.getOwnPropertyDescriptor(globalThis, 'MessageChannel'),
  );
  const channels = [],
    NativeMessageChannel = globalThis.MessageChannel;
  // Bundled React act() falls back to message ports; close them after the run.
  globalThis.MessageChannel = class extends NativeMessageChannel {
    constructor() {
      super();
      channels.push(this);
    }
  };
  const directory = await mkdtemp(join(tmpdir(), 'graphene-react-dom-'));
  try {
    const alias =
      version === 18
        ? {
            react: join(root, 'node_modules/react18'),
            'react-dom': join(root, 'node_modules/react-dom18'),
          }
        : {};
    const bundle = await build({
      stdin: { contents, resolveDir: root, sourcefile: 'react-dom-check.mjs' },
      alias,
      bundle: true,
      platform: 'node',
      format: 'cjs',
      write: false,
      define: { 'process.env.NODE_ENV': '"development"' },
    });
    const file = join(directory, 'test.cjs');
    await writeFile(file, bundle.outputFiles[0].text);
    return await run(createRequire(import.meta.url)(file));
  } finally {
    for (const channel of channels) {
      channel.port1.close();
      channel.port2.close();
    }
    dom.window.close();
    await rm(directory, { recursive: true, force: true });
    for (const [name, descriptor] of previous) {
      if (descriptor) Object.defineProperty(globalThis, name, descriptor);
      else delete globalThis[name];
    }
  }
}
