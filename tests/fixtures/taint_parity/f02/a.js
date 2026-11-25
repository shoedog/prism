import { exec } from 'child_process';
class Runner {
  run(x) {
    inner(x);
  }
}
function inner(c) {
  exec(c); // @SINK
}
function main(handler) {
  const t = getInput(); // @SRC
  handler.run(t);
}
