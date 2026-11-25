import { exec } from 'child_process';
function outer() {
  const t = getInput(); // @SRC
  function inner() {
    exec(t); // @SINK
  }
  inner();
}
outer();
