const cp = require('child_process');
function run(a) {
  cp.execSync('echo ' + a);
  eval(a);
  new Function(a);
  cp.exec(a);
}
module.exports = run;
