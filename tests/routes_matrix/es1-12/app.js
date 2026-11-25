const express = require('express');
const app = express();
function discover(req, res) { res.send('catalog'); }
app['m-search']('/catalog', discover);
module.exports = app;
