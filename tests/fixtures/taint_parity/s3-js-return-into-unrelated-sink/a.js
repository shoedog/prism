import { exec } from 'child_process';
function source() {
  const t = acquire(); // @SRC
  return t;
}
function a() { const x = source(); return x; }
function b() { const y = a(); return y; }
function entry() {
  const z = b();
  observe(z);
}
function other() {
  const z = 'fixed';
  exec(z); // @SINK
}
