import { exec } from 'child_process';
function pick(p) {
  log(p);
  observe(p.a); exec(p.b); // @SINK
}
function main() {
  const t = getInput(); // @SRC
  pick({ a: t, b: 'fixed' });
}
main();
