import { exec } from 'child_process';
function run(p) {
  const use = () => {
    exec(p.cmd); // @SINK
  };
  use();
}
function main() {
  const t = acquire(); // @SRC
  run({cmd: t});
}
main();
