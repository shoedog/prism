import { exec } from 'child_process';
function main() {
  const t = getInput(); // @SRC
  const u = t;
  exec(
    u, // @SINK
    t
  );
}
main();
