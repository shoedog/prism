const express = require('express');
const app = express();

function startExport(req, res) { res.sendStatus(204); }

app.post('/exports', startExport);

module.exports = app;
