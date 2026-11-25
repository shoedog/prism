import { exec } from 'child_process';
function run(p) {
  exec(p.key); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({key: t, key: 'fixed'});
}
main();
