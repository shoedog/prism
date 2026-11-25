const express = require('express');
const app = express();

function validate(scope) {
  return function checkScope(req, res, next) {
    if (req.get('X-Scope') === scope) return next();
    res.sendStatus(403);
  };
}

function h(req, res, next) { next(); }
function respond(req, res) { res.send('ready'); }

app.get('/', validate('r'), h, respond);

module.exports = app;
