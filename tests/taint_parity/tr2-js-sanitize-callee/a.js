import { exec } from 'child_process';
function run(x) {
  const y = escapeHtml(x);
  exec(y); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run(t);
}
main();
