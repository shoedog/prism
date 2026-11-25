import { exec } from 'child_process';
function run(x = 'fixed') {
  exec(x); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  const send = run;
  send(t);
}
main();
