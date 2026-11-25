import { exec } from 'child_process';
function id(x) { return x; }
function main() {
  const t = acquire(); // @SRC
  id(t);
  const y = id('fixed');
  exec(y); // @SINK
}
main();
