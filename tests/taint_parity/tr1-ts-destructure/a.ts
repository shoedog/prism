import { exec } from 'child_process';
function run({cmd}: {cmd: string}) {
  exec(cmd); // @SINK
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t});
}
main();
