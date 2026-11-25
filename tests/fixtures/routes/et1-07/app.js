const express = require('express');
const app = express();
app.use('/assets', express.static('public'));
module.exports = app;
