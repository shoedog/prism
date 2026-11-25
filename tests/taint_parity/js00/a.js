import { exec } from 'child_process';
function main() {
  const t = getInput(); // @SRC
  exec(t); // @SINK
}
main();
