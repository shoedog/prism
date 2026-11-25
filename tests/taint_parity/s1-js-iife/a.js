import { exec } from 'child_process';
function main() {
  const t = acquire(); // @SRC
  ((x) => {
    exec(x); // @SINK
  })(t);
}
main();
