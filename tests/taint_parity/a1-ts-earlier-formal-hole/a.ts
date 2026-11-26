import { exec } from 'child_process';
function run({enabled}: {enabled: boolean}, value: string) {
  exec(value); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  run({enabled: true}, t);
}
