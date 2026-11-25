const cp = require('child_process');
function run(cmd) {
  cp.execSync('echo ' + cmd);
}
function other(z) {
  eval(z);
}
module.exports = { run, other };
