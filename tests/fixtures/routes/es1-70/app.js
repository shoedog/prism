const express = require('express');
const app = express();
function auth(req, res, next) { next(); }
function audit(req, res, next) { next(); }
function finish(req, res) { res.send('result'); }
const checks = [auth, audit];
app.get('/result', ...checks, finish);
module.exports = app;
