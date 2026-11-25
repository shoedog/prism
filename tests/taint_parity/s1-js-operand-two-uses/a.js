import { exec } from 'child_process';
function main() {
  const t = acquire(); // @SRC
  const first = 'fixed';
  const second = t;
  exec(first); exec(second); // @SINK
}
main();
