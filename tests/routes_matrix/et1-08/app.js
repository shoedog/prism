const express = require('express');
const app = express();
function check(req, res, next) { next(); }
function show(req, res) { res.end(); }
app.route('/record').all(check).get(show);
module.exports = app;
