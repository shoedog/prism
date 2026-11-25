import { exec } from 'child_process';
declare function acquire(): string;
function run([cmd]: [string], enabled: boolean) {
  if (enabled) {
    exec(cmd); // @SINK
  }
}
function main() {
  let t = acquire(); // @SRC
  run([t], false); t = 'fixed'; run([t], true);
}
main();
