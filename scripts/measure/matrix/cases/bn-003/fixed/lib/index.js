const cp = require('child_process');
function run(cmd) {
  cp.execSync('echo fixed');
  eval('1 + ' + cmd);
}
module.exports = run;
