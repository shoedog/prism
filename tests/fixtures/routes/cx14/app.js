const express = require('express');
const app = express();

function patchPreferences(req, res) { res.sendStatus(204); }

app.patch('/preferences', patchPreferences);

module.exports = app;
