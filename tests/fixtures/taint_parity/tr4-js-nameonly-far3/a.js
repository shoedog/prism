import { exec } from 'child_process';
function last(x) { exec(x); } // @SINK
function third(x) { last(x); }
function second(x) { third(x); }
class Handler { run(x) { second(x); } }
function main(handler) {
  const t = acquire(); // @SRC
  handler.run(t);
}
main(new Handler());
