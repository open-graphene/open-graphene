import { withReactDom } from './react-dom-harness.mjs';
for (const version of [18, 19]) {
  const result = await withReactDom(version,
    `export {runReactChecks} from './tests/react.browser.mjs'; export {runMockRoomE2E} from './tests/react-room-mock.mjs'; export {version} from 'react';`,
    async module => ({ version: module.version, checks: await module.runReactChecks(), roomE2E: await module.runMockRoomE2E() }));
  console.log('React DOM matrix:', result);
}
