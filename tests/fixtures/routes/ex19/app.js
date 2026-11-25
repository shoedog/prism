const express = require('express');
const app = express();

const r = express.Router();

function users(req, res) { res.sendStatus(204); }
r.get('/users', users);
app.use('/api', r);

module.exports = app;
