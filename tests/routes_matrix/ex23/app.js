const express = require('express');
const app = express();

const r = express.Router();
const PREFIX = '/preview';

function users(req, res) { res.sendStatus(204); }
r.get('/users', users);
app.use(PREFIX, r);

module.exports = app;
