import { exec } from 'child_process';
function main() {
  const t = acquire(), u = acquire(); // @SRC
  exec(u); // @SINK
}
main();
