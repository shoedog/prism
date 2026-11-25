const express = require('express');
const app = express();
function finish(req, res) { res.send('result'); }
app['get']('/result', finish);
module.exports = app;
