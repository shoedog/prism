import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const defaults = require('config');
export function load() {
  const port = defaults.get('ui.port');
  return port;
}
