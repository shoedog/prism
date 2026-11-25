const express = require('express');
const app = express();
const portal = express.Router();

function history(req, res) { res.json([]); }

portal.get('/history', history);
app.use(/^\/branch-[0-9]+/, portal);

module.exports = app;
