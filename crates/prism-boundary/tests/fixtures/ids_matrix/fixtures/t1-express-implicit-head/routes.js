const express = require('express');
const app = express();
app.get('/forge', (req, res) => res.send('ready'));
module.exports = app;
