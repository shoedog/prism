import { exec } from 'child_process';
function main() {
  const t = getInput(); // @SRC
  const arr = [t];
  arr.forEach((v) => exec(v)); // @SINK
}
main();
