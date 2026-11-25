import { exec } from 'child_process';
declare function acquire(): string;
class Runner {
  constructor(x: string) {
    exec(x); // @SINK
  }
}
function main() {
  const t = acquire(); // @SRC
  new Runner(t);
}
main();
