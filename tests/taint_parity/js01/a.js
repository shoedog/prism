import { exec } from 'child_process';
function run(c) {
  exec(c); // @SINK
}
function main() {
  const t = getInput(); // @SRC
  run(t);
}
main();
