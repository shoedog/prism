const express = require('express');
const app = express();

const r = express.Router();

function h(req, res) { res.sendStatus(204); }
app.use('/u', r);
r.get('/', h);

module.exports = app;
