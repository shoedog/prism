const express = require('express');
const reports = require('./reports');
const support = express.Router();
support.use('/analytics', reports);
module.exports = support;
