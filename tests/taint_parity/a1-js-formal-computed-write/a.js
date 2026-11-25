import { exec } from 'child_process';
function pick(p) {
  const k = "other";
  p[k] = "fixed";
  exec(p.key); // @SINK
}
function entry() {
  const t = acquire(); // @SRC
  pick({key: t});
}
