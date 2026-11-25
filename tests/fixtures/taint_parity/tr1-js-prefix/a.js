import { exec } from 'child_process';
function run(p) {
  exec(p.outer.key); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({outer: {key: t}});
}
main();
