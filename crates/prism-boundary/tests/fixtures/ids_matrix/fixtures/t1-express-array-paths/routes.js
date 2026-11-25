const express = require('express');
const app = express();
app.post(['/loom', '/spindle'], (req, res) => res.send('ready'));
module.exports = app;
