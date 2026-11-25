import { exec } from 'child_process';
function log(m) { console.log(m); }
function g(x) {
  exec(x); // @SINK
}
function main() {
  const t = getInput();
  log(t); g(t); // @SRC
}
main();
