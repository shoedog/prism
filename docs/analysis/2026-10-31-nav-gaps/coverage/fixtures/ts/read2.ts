import { readFileSync } from 'fs';
import { join } from 'path';
export function readConfig2(opts: { name: string }) {
  const p = join('/etc/app', opts.name);
  return readFileSync(p, 'utf8');
}
