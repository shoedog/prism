import { exec } from 'child_process';
function pick(p) {
  exec(p.__proto__); // @SINK
}
function entry() {
  const __proto__ = acquire(); // @SRC
  pick({__proto__});
}
