import { exec } from 'child_process';
function run<T extends string>(x: T) {
  exec(x); // @SINK
}
function main() {
  const t: string = acquire(); // @SRC
  run<string>(t);
}
main();
