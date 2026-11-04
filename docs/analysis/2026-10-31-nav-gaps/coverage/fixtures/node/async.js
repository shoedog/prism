const { exec } = require('child_process');
const fs = require('fs');
function later(cb) { setTimeout(cb, 10); }
function handler(req, res) {
  const cmd = req.body.cmd;
  later(() => { exec(cmd); });
  Promise.resolve(req.body.file).then((f) => { fs.readFileSync(f); });
  res.end();
}
async function handler2(req, res) {
  const f = await Promise.resolve(req.query.f);
  fs.readFileSync(f);
  res.end();
}
module.exports = { handler, handler2 };
