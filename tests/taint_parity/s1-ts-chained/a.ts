import { exec } from 'child_process';
declare function acquire(): string;
function run(x: string) {
  exec(x); // @SINK
}
function select(): (x: string) => void {
  return run;
}
function main() {
  const t = acquire(); // @SRC
  select()(t);
}
main();
