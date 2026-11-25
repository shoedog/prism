const express = require('express');
const app = express();

function a(req, res) { res.send('compact'); }
function b(req, res) { res.send('detailed'); }

function wrap(compact, detailed) {
  return function chooseView(req, res) {
    const view = req.query.detail ? detailed : compact;
    view(req, res);
  };
}

app.get('/x', wrap(a, b));

module.exports = app;
