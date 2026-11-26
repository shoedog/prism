const express = require('express');
const app = express();

function revision(req, res) { res.sendStatus(204); }

app.route('/revision').all(revision);

module.exports = app;
