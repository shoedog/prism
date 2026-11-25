const express = require('express');
const app = express();
function load(req, res, next) { next(); }
function recover(err, req, res, next) { res.status(500).send('unavailable'); }
function respond(req, res) { res.send('ready'); }
app.get('/ready', load, recover, respond);
module.exports = app;
