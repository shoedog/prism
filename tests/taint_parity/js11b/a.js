import { exec } from 'child_process';
function run(c) {
  exec(c); // @SINK
}
function main() {
  const t = getInput(); // @SRC
  const u = 'safe';
  run(t);
  run(u);
}
main();
