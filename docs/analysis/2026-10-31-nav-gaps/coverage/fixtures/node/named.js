const { runCmd } = require('./lib/cmd');
const merge = require('./lib/merge');
const cmdmod = require('./lib/cmd');
function viaNamed(req, res) {
  runCmd(req.body.c);
  merge({}, req.body);
  cmdmod.runCmd(req.body.d);
  res.end();
}
module.exports = { viaNamed };
