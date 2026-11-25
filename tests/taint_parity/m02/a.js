import { exec } from 'child_process';
function pick(p) {
  log(p);
  exec(p.b); observe(p.a); // @SINK
}
function main() {
  const t = getInput(); // @SRC
  pick({ a: t, b: 'fixed' });
}
main();
