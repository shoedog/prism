const cp = require('child_process');
function run(a, b) {
  cp.execSync('echo ' + a); cp.execSync('echo ' + b);
}
module.exports = run;
