const express = require('express');
const app = express();

function replacePolicy(req, res) { res.sendStatus(204); }

app.put('/policies/current', replacePolicy);

module.exports = app;
