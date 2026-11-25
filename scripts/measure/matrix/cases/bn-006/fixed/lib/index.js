const cp = require('child_process');
function runCommand(cmd) {
  const full = 'echo ' + cmd;
  cp.execSync(full);
}
module.exports = runCommand;
