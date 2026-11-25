const cp = require('child_process');
function run(cmd) {
  return helper('echo ' + cmd);
}
function helper(full) {
  cp.execSync(full);
}
module.exports = run;
