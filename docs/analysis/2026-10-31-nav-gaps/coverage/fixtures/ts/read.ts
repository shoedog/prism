import { readFileSync } from 'fs';
import { join } from 'path';
export function readConfig(opts: { name: string }) {
  return readFileSync(join('/etc/app', opts.name), 'utf8');
}
