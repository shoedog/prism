import { exec } from 'child_process';
class Runner {
  run() { exec(this.value); } // @SINK
}
function main() {
  const t = acquire(); // @SRC
  const r = new Runner();
  r.value = t;
  r.run();
}
main();
