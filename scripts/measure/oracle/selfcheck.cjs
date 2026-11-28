'use strict';

// Bootstrap only. Pin verification and the structured guard handshake precede
// the separate Node worker that loads TypeScript and the question.
const { spawnSync } = require('node:child_process');
const path = require('node:path');

if (require.main === module) {
  const child = spawnSync('python3', ['-m', 'measure.oracle', ...process.argv.slice(2)], {
    cwd: process.cwd(),
    env: { ...process.env, PYTHONPATH: path.resolve(__dirname, '../..') },
    stdio: 'inherit',
  });
  if (child.error) console.error(child.error.message);
  process.exitCode = child.status ?? 1;
}
