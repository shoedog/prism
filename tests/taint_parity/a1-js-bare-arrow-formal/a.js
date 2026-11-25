import { exec } from 'child_process';
const run = value => {
  exec(value); // @SINK
};
function entry() {
  const t = acquire(); // @SRC
  run(t);
}
