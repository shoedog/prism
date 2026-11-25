import { exec } from 'child_process';
function main() {
  const t = input(); // @SRC
  const y = 'fixed';
  const x = t;
  exec(y, x); // @SINK
}
