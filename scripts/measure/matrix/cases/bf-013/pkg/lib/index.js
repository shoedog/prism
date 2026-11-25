const cp = require('child_process');
function run(cmd) {
  eval('x' + cmd);
  cp.execSync('echo ' + cmd);
}
module.exports = run;
