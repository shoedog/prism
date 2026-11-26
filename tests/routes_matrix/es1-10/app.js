const express = require('express');
const app = express();
function check(req, res, next) { next(); }
function download(req, res) { res.send('archive'); }
const archive = app.route('/archive');
archive.get(check);
archive.get(download);
module.exports = app;
