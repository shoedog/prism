const express = require('express');
const reports = require('./reports');
const staff = express.Router();
staff.use('/reports', reports);
module.exports = staff;
