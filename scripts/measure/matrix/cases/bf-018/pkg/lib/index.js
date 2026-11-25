const cp = require('child_process');
function run(cmd) {
  cp.execSync('echo ' + cmd);
}
module.exports = run;
