import { exec } from 'child_process';
function id(x) { return x; }
function wrap(x) { return id(x); }
function main() {
  const t = acquire(); // @SRC
  const y = wrap(t);
  exec(y); // @SINK
}
main();
