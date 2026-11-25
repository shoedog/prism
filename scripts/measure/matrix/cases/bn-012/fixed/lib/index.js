const cp = require('child_process');
function quote(s) {
  return "'" + s.replace(/'/g, "'\\''") + "'";
}
function run(a, b) {
  cp.execSync('echo ' + quote(a) + b);
}
module.exports = run;
