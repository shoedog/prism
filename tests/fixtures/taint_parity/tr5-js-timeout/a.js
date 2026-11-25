import { exec } from 'child_process';
function main() {
  const t = acquire(); // @SRC
  setTimeout(() => { exec(t); /* @SINK */ }, 0);
}
main();
