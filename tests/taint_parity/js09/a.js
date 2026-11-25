import { exec } from 'child_process';
function main() {
  const o = {};
  const t = getInput(); // @SRC
  o.x = t;
  exec(o.x); // @SINK
}
main();
