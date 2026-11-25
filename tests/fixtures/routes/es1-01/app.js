const express = require('express');
const staff = require('./staff');
const support = require('./support');
const app = express();
app.use('/staff', staff);
app.use('/support', support);
module.exports = app;
