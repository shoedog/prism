const cp = require('child_process');
function run(cmd) {
  const full = 'echo ' + cmd;
  cp.execSync(full);
}
module.exports = run;
