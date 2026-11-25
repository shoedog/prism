const nodeProcess = require('node:process');
function load() {
  const label = nodeProcess.env.PREVIEW_LABEL;
  return label;
}
