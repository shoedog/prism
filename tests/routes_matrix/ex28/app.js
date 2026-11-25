const express = require('express');
const app = express();

function a(req, res) { res.sendStatus(204); }
function b(req, res) { res.sendStatus(201); }
function c(req, res) { res.sendStatus(204); }

app.get('env');
app.route('/c').get(a).post(b).head(c);

module.exports = app;
