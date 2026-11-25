const cp = require('child_process');
function exec(full) {
  cp.execSync(full);
}
function run(cmd) {
  if (cmd.length > 100) throw new Error('too long');
  exec(cmd);
}
module.exports = run;
