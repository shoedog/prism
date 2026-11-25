const express = require('express');
const jobs = express.Router();
function queue(req, res) { res.send('queue'); }
jobs.get('/queue', queue);
module.exports = jobs;
