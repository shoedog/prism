import { exec } from 'child_process';
function make() {
  const t = acquire(); // @SRC
  return t;
}
function consume(x) { exec(x); } // @SINK
function main() {
  const y = make();
  consume(y);
}
main();
