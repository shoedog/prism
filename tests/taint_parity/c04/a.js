import { exec } from 'child_process';
function main() {
  const t = input(); // @SRC
  const x = t;
  log(x); exec(x); // @SINK
}
