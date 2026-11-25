import { exec } from 'child_process';
function id(x) {
  return x;
}
function main() {
  const t = getInput(); // @SRC
  const y = id(
    t
  );
  exec(y); // @SINK
}
main();
