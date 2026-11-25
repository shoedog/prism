const express = require('express');
const app = express();
function first(req, res, next) { next(); }
function last(req, res) { res.end('done'); }
app.get('/a', first, last); app.get('/b', last);
module.exports = app;
