const cp = require('child_process');
function wrap() {
  function run(x) {
    return x;
  }
  return run;
}
function run(cmd) {
  const full = 'echo ' + cmd;
  cp.execSync(full);
}
module.exports = run;
