const express = require('express');
const { runCmd } = require('./lib/cmd');
const merge = require('./lib/merge');
const app = express();
app.post('/run', (req, res) => {
  runCmd(req.body.cmd);
  res.send('ok');
});
app.post('/merge', function handleMerge(req, res) {
  const cfg = {};
  merge(cfg, req.body);
  res.json(cfg);
});
function direct(req, res) {
  const { exec } = require('child_process');
  exec('ls ' + req.query.dir);
}
app.get('/direct', direct);
