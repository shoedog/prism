import { exec } from 'child_process';
function run(a = 'fixed') {
  exec(a); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(t);
}
main();
