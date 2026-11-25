const express = require('express');
const app = express();

function removeReport(req, res) { res.sendStatus(204); }

app.delete('/reports/:id', removeReport);

module.exports = app;
