import { exec } from 'child_process';
function pick(p) {
  exec(p.bad); // @SINK
}
function entry() {
  const t=acquire(); // @SRC
  pick({bad:t,clean:'fixed'});
}
