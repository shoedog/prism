import { exec } from 'child_process';
class Runner {
  run(x) {
    inner(x);
  }
}
function inner(c) {
  exec(c); // @SINK
}
function pick(p, handler) {
  handler.run(p.a);
}
function main(handler) {
  const t = input(); // @SRC
  pick({ a: t }, handler);
}
