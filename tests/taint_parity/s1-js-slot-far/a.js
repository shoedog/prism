import { exec } from 'child_process';
function run(x = 'fixed') {
  first(x);
}
function first(y) {
  last(y);
}
function last(z) {
  exec(z); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(t);
}
main();
