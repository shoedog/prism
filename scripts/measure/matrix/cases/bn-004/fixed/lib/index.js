const cp = require('child_process');
function run(cmd) {
  const full = 'printf ' + cmd;
  cp.execSync(full);
}
module.exports = run;
