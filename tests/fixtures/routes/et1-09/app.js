const express = require('express');
const app = express();
function show(req, res) { res.end(); }
app.get(['/a', '/b'], show);
module.exports = app;
