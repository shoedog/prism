const express = require('express');
const app = express();
function load(req, res, next, id) { next(); }
function read(req, res) { res.end(); }
app.param('id', load);
app.get('/items/:id', read);
module.exports = app;
