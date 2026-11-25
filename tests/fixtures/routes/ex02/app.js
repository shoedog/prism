const express = require('express');
const app = express();

app.get('/x', (req, res) => res.sendStatus(204));

module.exports = app;
