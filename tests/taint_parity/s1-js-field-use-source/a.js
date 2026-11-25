import { exec } from 'child_process';
function run(x) {
  exec(x); // @SINK
}
function main() {
  const record = {cmd: 'fixed'};
  record.cmd = acquire();
  run(
    record.cmd // @SRC
  );
}
main();
