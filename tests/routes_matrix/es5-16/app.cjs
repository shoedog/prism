const express = require('express');
const app = express();
const catalog = express.Router();

function browse(req, res) { res.send('catalog'); }

catalog.get('/catalog', browse);
app.get('/catalog', catalog);
app.use('/library', catalog);

module.exports = app;
